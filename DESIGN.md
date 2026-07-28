# aicommit — Design Document

## Overview

`aicommit` is a CLI tool that reads staged Git diffs, sends them to an AI
provider, and produces structured Conventional Commit messages. It's written
in Rust with a focus on speed (no subprocess calls, native `git2` bindings,
async HTTP) and privacy (Ollama by default, zero cloud by default).

### Guiding principles

1. **Local-first** — Ollama is the default provider; no API key needed.
2. **Fast** — native Rust, libgit2 in-process, async HTTP, sub-second on small repos.
3. **Atomic** — large diffs are split into multiple focused commits, one per
   logical domain.
4. **Convention over configuration** — Conventional Commits out of the box,
   zero config needed to start.

---

## Module map

```
src/
├── main.rs         CLI entry point (clap), subcommand dispatch
├── lib.rs          Re-exports all modules
├── config.rs       Config loading, merging, resolution, CLI config command
├── git.rs          git2 wrapper: diff, stage, commit, undo, status checks
├── llm/
│   ├── mod.rs      AnyProvider enum, factory, retry logic, generate helper
│   ├── ollama.rs   Ollama /api/chat client
│   ├── openai_compat.rs  OpenAI-compatible /chat/completions client
│   ├── anthropic.rs      Anthropic Messages API client
│   ├── gemini.rs         Gemini generateContent client
│   └── mock.rs           Mock provider for testing
├── parser.rs       LLM response parsing (JSON → text fallback)
├── prompt.rs       System + user prompt construction
├── splitter.rs     Atomic commit grouping by directory
├── interactive.rs  Interactive commit selection + message editing
├── display.rs      Terminal box rendering (╭─ ╰─ │)
└── banner.rs       First-run ASCII banner
```

---

## Data flow

### `aicommit` (default workflow)

```
User runs `aicommit`
    │
    ▼
main()
    ├── banner::maybe_print()         (once, first launch only)
    ├── Cli::parse()                  (clap)
    ├── git::ensure_in_repo()         (fail fast if not in a git repo)
    │
    ▼
commit_workflow()
    │
    ├── config::load(cli)             (CLI → env → .aicommit.toml → global → defaults)
    ├── GitRepo::from_current_dir()
    ├── GitRepo::is_operation_in_progress()
    │      └── merge/rebase/bisect/cherry-pick → abort with message
    │
    ├── GitRepo::staged_diff()
    │      └── empty + no --no-stage → GitRepo::stage_all() → diff again
    │      └── still empty → "nothing to commit" message
    │
    ├── llm::build_provider(&cfg)
    │
    ├── should_treat_as_single(&diff)?
    │   ├── YES ──→ llm::generate_commit_message() ──→ parse ──→ commit
    │   │              ├── chat_with_retry() (3 attempts)
    │   │              ├── parse_commit_message() (JSON → text fallback)
    │   │              └── format_with_template(cfg.commit_template)
    │   │
    │   └── NO ───→ splitter::group_by_directory(diff, max, exclude)
    │                for each group:
    │                  ├── git::diff_for_group(group)
    │                  ├── llm::generate_commit_message()
    │                  ├── parse ──→ format_with_template
    │                  └── commit (scoped to group paths)
    │
    └── do_commit()
           ├── dry-run → print "Would commit: ..."
           └── real   → GitRepo::commit(message, paths)
```

### `aicommit undo`

```
main() → Cli::parse() → Command::Undo
    git::ensure_in_repo()
    GitRepo::undo_last_commit()    (git reset --soft HEAD~1)
```

### `aicommit config --provider X --api-key Y`

```
main() → Cli::parse() → Command::Config
    config::handle_config_command()
        ├── reads existing global config
        ├── overwrites provider section (api_key, model, base_url)
        └── writes to ~/.config/aicommit/config.toml
```

---

## Configuration resolution

Priority (highest → lowest):

1. **CLI flags** (`--provider`, `--model`, `--lang`, `--api-key`, `--base-url`, `--timeout`)
2. **Environment variables** (per-provider API keys from `ProviderKind::env_var()`)
3. **Project config** (`./.aicommit.toml`)
4. **Global config** (`~/.config/aicommit/config.toml`)
5. **Hardcoded defaults**

### Provider auto-detection

If no provider is specified via CLI or config file, `resolve_provider()` tries:

1. Check if Ollama is reachable (`GET /api/tags`, 3s timeout) → use Ollama
2. Scan env vars for any known API key → use that provider
3. Error with setup instructions

### Config merging

`ConfigFile` uses `Option<T>` for all fields. The `merge_files()` function
overlays project config on top of global config by replacing `Some` values.
CLI overrides are applied by chaining `.or()` / `.unwrap_or()` in `load()`.

**Key decision**: Empty/invalid values in a lower-priority file never
overwrite valid values from a higher-priority file, because all fields are
`Option` and only `Some` values propagate.

---

## LLM provider architecture

### Dispatch enum (no trait objects)

Providers use a flat enum + match dispatch rather than `Box<dyn ProviderTrait>`:

```rust
pub enum AnyProvider {
    Mock(mock::MockProvider),
    Ollama(ollama::OllamaProvider),
    OpenAiCompat(openai_compat::OpenAiCompatProvider),
    Anthropic(anthropic::AnthropicProvider),
    Gemini(gemini::GeminiProvider),
}
```

**Rationale**: Simplicity, no vtable overhead, no `async_trait` dependency,
exhaustive matching in the IDE.

### Provider interfaces

Every provider exposes two methods through the enum:

| Method | Signature | Purpose |
|---|---|---|
| `chat` | `(&self, system: &str, user: &str) -> Result<String>` | Send prompts, return raw response text |
| `list_models` | `(&self) -> Result<Vec<String>>` | List available models (used by `--list-models`) |

### Retry strategy: `chat_with_retry()`

- **3 attempts** with exponential backoff (1s, 2s, 4s).
- Retries on: 429 Too Many Requests, 503 Service Unavailable, timeout,
  connection errors (dns, refused, reset).
- Non-transient errors (401, 403, 400, 404) fail fast with a clear message.

### Parse retry: `do_generate()`

After a successful API call, if the response can't be parsed as a
`CommitMessage`, a stricter repair prompt is sent (one retry). The repair
prompt includes the JSON schema and the parse error.

---

## Commit splitting algorithm

### `group_by_directory(diff, max_groups, exclude_patterns)`

1. Extract file paths from `diff --git a/X b/Y` headers.
2. Skip paths matching any glob in `exclude_patterns` (uses `glob::Pattern`).
3. Assign each file to a bucket via `group_key(path)`:
   - Repo root files → `"root"`
   - `src/auth/login.rs` (3+ components) → `"auth"` (2nd component)
   - `src/main.rs` (2 components) → `"src"` (1st component)
   - `assets/icons/close.svg` → `"icons"` (2nd component)
4. If all files → one bucket, return single group named `"root"`.
5. If buckets > `max_groups` (default 3), keep largest `max_groups - 1`
   groups and merge the rest into `"misc"`.
6. Sort groups by file count descending.

### `should_treat_as_single(diff)`

Returns `true` if diff has ≤ 1 `diff --git` header (one file changed).
Bypasses the splitter entirely.

---

## Parsing strategy

### Two-phase parser

1. **JSON first**:
   - Strip markdown fences (` ```json `, ` ``` `) and surrounding prose.
   - Find first `{` and last `}`, deserialize with `serde_json`.
   - Schema: `{ "type": str, "scope": str?, "description": str, "body": str? }`

2. **Text fallback** (if JSON parsing fails):
   - Parse `type(scope): description` from the first non-empty line.
   - Remaining lines become the body.

3. **Validation**:
   - `type` must be in `ALLOWED_TYPES`: `feat`, `fix`, `docs`, `style`,
     `refactor`, `perf`, `test`, `build`, `ci`, `chore`.
   - `description` ≤ 120 chars, non-empty.

### Template formatting

`CommitMessage::format_with_template(template)` replaces placeholders in the
commit template from `.aicommit.toml`:

| Placeholder | Source |
|---|---|
| `{type}` | `kind` field |
| `{scope}` | `scope` field (empty string if None) |
| `{description}` | `description` field |
| `{body}` | `body` field (empty string if None) |
| `{emoji}` | Emoji mapped from `kind` (feat → ✨, fix → 🐛, etc.) |

Default template: `{type}{scope}: {description}`

If the template does not contain `{body}` but a body exists, it is
automatically appended after a blank line.

---

## Prompt design

### System prompt

Builds a strict instruction set including:

- Conventional Commits specification
- Allowed types list
- JSON output schema with field descriptions
- Formatting rules (imperative, lowercase, max 72 chars, no trailing period)
- Optional language clause (`Write in {lang}.`)

### User prompt

Wraps the compressed diff in a markdown code block with the instruction
to produce exactly one commit message in the required JSON schema.

### Diff compression (`compress_diff`)

| Limit | Value |
|---|---|
| Per-file max | 8,000 bytes |
| Total max | 30,000 bytes |
| Truncation marker | `... [truncated by aicommit] ...` |

Binary file lines are dropped.

---

## Git integration

### `GitRepo` (wraps `git2::Repository`)

| Operation | git2 API |
|---|---|
| Open repo | `Repository::discover(path)` |
| Head tree | `repo.head()?.peel_to_tree()` or empty tree `4b825dc642` |
| Staged diff | `diff_tree_to_index(Some(&head), &index, None)` |
| Stage all | `index.add_all(["*"], DEFAULT, None)` |
| Stage paths | `index.add_path(p)` for each |
| Commit | `index.write_tree()` → `repo.find_tree()` → `repo.commit()` |
| Undo last | `repo.head().peel_to_commit()` → `repo.reset(parent, Soft)` |
| In-progress check | Detect `MERGE_HEAD`, `REBASE_HEAD`, `BISECT_LOG`, `CHERRY_PICK_HEAD` |
| Per-group diff | `diff_tree_to_index()` with `pathspec` filter |

### Signature

Uses `repo.signature()` (reads `user.name` / `user.email` from Git config).
Falls back to `"aicommit" <aicommit@users.noreply.github.com>`.

---

## Interactive mode

### `select_commits(groups, messages)`

- `dialoguer::MultiSelect` with entries formatted as `[group_name]  message`.
- Empty selection defaults to ALL groups (users who press Enter without
  selecting anything commit everything).

### `edit_message(msg)`

- `dialoguer::Input` with `with_initial_text(msg)`.
- Returns the edited message.

---

## CLI structure

### Flags

| Flag | Type | Default |
|---|---|---|
| `--single` / `-1` | bool | `false` |
| `--interactive` / `-i` | bool | `false` |
| `--dry-run` / `-n` / `--print-only` | bool | `false` |
| `--no-stage` | bool | `false` |
| `--provider` | `Option<String>` | `None` |
| `--model` | `Option<String>` | `None` |
| `--base-url` | `Option<String>` | `None` |
| `--lang` | `Option<String>` | `None` |
| `--api-key` | `Option<String>` | `None` |
| `--timeout` | `u64` | `120` |
| `--list-models` | bool | `false` |

### Subcommands

| Command | Action |
|---|---|
| `config --provider X --api-key Y` | Write API key to global config |
| `config --show` | Print resolved config |
| `undo` | `git reset --soft HEAD~1` |

---

## Known design limitations

1. **Unicode box alignment**: `display.rs` uses byte length (`.len()`) instead
   of display width for padding. Multi-byte characters and emoji break the
   box borders.

2. **Banner version hardcoded**: `banner.rs` contains a hardcoded version
   string (`v0.2.0`) that drifts from `Cargo.toml`.

3. **Interactive panic on Ctrl+C**: `dialoguer::MultiSelect::interact()`
   unwraps rather than handling interruption gracefully.

4. **No streaming**: LLM responses are awaited in full before displaying.
   Streaming (SSE) would improve perceived latency.

5. **No diff preview**: The splitter shows group names but not the actual
   diff content before committing.

---

## Security

- `#![deny(unsafe_code)]` at the crate root.
- API keys stored in `~/.config/aicommit/config.toml` (file permissions set
  to 0600 on Unix).
- No telemetry, no analytics, no network calls except to the chosen provider.

---

## Dependencies

| Crate | Purpose |
|---|---|
| `clap` | CLI argument parsing |
| `git2` | libgit2 bindings (in-process git operations) |
| `reqwest` | Async HTTP client (TLS via `rustls`) |
| `tokio` | Async runtime |
| `serde` / `serde_json` | JSON serialization/deserialization |
| `toml` | Config file parsing |
| `colored` | Terminal coloring |
| `dialoguer` | Interactive prompts (MultiSelect, Input) |
| `anyhow` / `thiserror` | Error handling |
| `glob` | Exclude pattern matching in splitter |
| `dirs` | Platform config directory paths |
| `async-trait` | Trait support (currently unused — provider dispatch is enum-based) |
