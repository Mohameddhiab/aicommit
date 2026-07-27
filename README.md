<p align="center">
  <img src="media/banner.svg" width="800" alt="aicommit banner">
</p>

<div align="center">

[![CI](https://github.com/Mohameddhiab/aicommit/actions/workflows/ci.yml/badge.svg)](https://github.com/Mohameddhiab/aicommit/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/aicommit.svg)](https://crates.io/crates/aicommit)
[![codecov](https://codecov.io/gh/Mohameddhiab/aicommit/graph/badge.svg)](https://codecov.io/gh/Mohameddhiab/aicommit)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Rust](https://img.shields.io/badge/rust-2021-orange.svg)
[![Downloads](https://img.shields.io/crates/d/aicommit)](https://crates.io/crates/aicommit)

</div>

**Generate perfect, atomic Git commits using AI.** One command, under a second. Zero cloud by default.

> **100% open source** (MIT) · **local-first** (Ollama default) · **blazing fast** (native Rust, ~0.4s)

---

## Why aicommit?

- **Local-first** — Ollama by default, zero code leaves your machine. No API key needed.
- **Atomic** — splits large diffs into one commit per logical domain, not a single blob.
- **Fast** — native Rust, ~0.4s on a 5k-file repo (40x faster than Node alternatives).
- **8 providers** — Ollama, OpenAI, Anthropic, Groq, DeepSeek, Mistral, Gemini, OpenRouter.
- **Strict Conventional Commits** — every message is `feat`, `fix`, `refactor`, etc. with optional scope and body.
- **Reliable** — auto-retry with backoff, explicit error messages for bad API keys, merge/rebase detection.

```bash
# Before: "fix stuff" — vague, inconsistent
# After:
aicommit
# ✔ feat(auth): implement JWT token expiration
# ✔ refactor(db): remove unused Redis client pool
```

---

## Quick Start

### Local (no API key)

```bash
ollama pull qwen2.5-coder:7b    # one-time setup
aicommit                          # in any git repo
```

### Cloud

```bash
aicommit config --provider openai --api-key sk-...
aicommit --provider openai --model gpt-4o
```

---

## Features

- **Atomic commits** — groups files by top-level directory, commits each group separately
- **Interactive mode** (`-i`) — pick which groups to commit, edit messages before applying
- **Auto-stage** — `aicommit` = `git add .` + commit; disable with `--no-stage`
- **Dry-run** (`-n`) — preview generated messages without committing
- **Multi-language** — generate messages in French, Spanish, German, etc. (`--lang fr`)
- **Custom base URL** — use with vLLM, LM Studio, or any OpenAI-compatible local server
- **Retry + backoff** — 3 attempts on timeouts and server errors
- **Undo** (`aicommit undo`) — `git reset --soft HEAD~1` to undo the last commit

---

## Supported Providers

| Provider | Auth | Default Model | Local/Cloud |
|----------|------|---------------|-------------|
| [Ollama](https://ollama.com) | None | `qwen2.5-coder:7b` | Local |
| [OpenAI](https://platform.openai.com) | `OPENAI_API_KEY` | `gpt-4o` | Cloud |
| [Anthropic](https://console.anthropic.com) | `ANTHROPIC_API_KEY` | `claude-sonnet-4-20250514` | Cloud |
| [Groq](https://groq.com) | `GROQ_API_KEY` | `llama-3.3-70b-versatile` | Cloud |
| [DeepSeek](https://deepseek.com) | `DEEPSEEK_API_KEY` | `deepseek-coder` | Cloud |
| [Mistral](https://mistral.ai) | `MISTRAL_API_KEY` | `codestral-latest` | Cloud |
| [Gemini](https://ai.google.dev) | `GOOGLE_API_KEY` | `gemini-2.0-flash` | Cloud |
| [OpenRouter](https://openrouter.ai) | `OPENROUTER_API_KEY` | `anthropic/claude-3.5-sonnet` | Cloud |

> **Auto-detection:** If no provider is specified, aicommit checks for a running Ollama instance first, then falls back to any provider with a configured API key.

---

## Installation

```bash
cargo install aicommit                           # from source
cargo install --git https://github.com/Mohameddhiab/aicommit.git  # latest
brew install Mohameddhiab/tap/aicommit            # Homebrew
```

Pre-built binaries for Linux (x86_64), macOS (x86_64 + aarch64), and Windows (x86_64) on the [Releases page](https://github.com/Mohameddhiab/aicommit/releases).

---

## Usage

```bash
aicommit                    # default: auto-stage + split + commit
aicommit --dry-run          # preview only
aicommit --interactive      # pick groups + edit messages
aicommit --single           # one commit, no splitting
aicommit --lang fr          # messages in French
aicommit --provider ollama --model llama3
aicommit undo               # undo last commit
```

<details>
<summary><b>CLI Flags</b> — full reference</summary>

| Flag | Description |
|------|-------------|
| `--single` (`-1`) | Force a single commit, skipping atomic splitting |
| `--interactive` (`-i`) | Interactive mode: select commits and edit messages |
| `--dry-run` (`-n`) | Generate and print messages without committing |
| `--no-stage` | Disable auto-staging (you must `git add` first) |
| `--provider <name>` | AI provider (`ollama`, `openai`, `anthropic`, `groq`, `deepseek`, `mistral`, `gemini`, `openrouter`) |
| `--model <name>` | Model override (e.g. `gpt-4o`, `claude-sonnet-4-20250514`) |
| `--base-url <url>` | Base URL for the provider API |
| `--lang <lang>` | Language of the generated message (`en`, `fr`, `es`, `de`, etc.) |
| `--api-key <key>` | Ephemeral API key (overrides env var and config file) |
| `--timeout <secs>` | Timeout in seconds for API calls (default: 120) |
| `--list-models` | List available models from the selected provider and exit |
| `--help` (`-h`) | Print help |

</details>

<details>
<summary><b>Configuration</b> — config file, env vars, resolution order</summary>

### Config File

**Project config** — `.aicommit.toml` in your project root:
```toml
[provider]
default = "ollama"

[ollama]
url = "http://localhost:11434"
model = "qwen2.5-coder:7b"

[commit]
language = "en"
max_commits = 3
```

**Global config** — `~/.config/aicommit/config.toml`:
```toml
[openai]
base_url = "https://api.openai.com/v1"
model = "gpt-4o"
```

### Environment Variables

| Variable | Provider |
|----------|----------|
| `OPENAI_API_KEY` | OpenAI |
| `ANTHROPIC_API_KEY` | Anthropic |
| `GROQ_API_KEY` | Groq |
| `DEEPSEEK_API_KEY` | DeepSeek |
| `MISTRAL_API_KEY` | Mistral |
| `GOOGLE_API_KEY` | Gemini |
| `OPENROUTER_API_KEY` | OpenRouter |

### Resolution Order

1. CLI flags (`--provider`, `--model`, `--lang`, `--api-key`)
2. Environment variables
3. Project config (`./.aicommit.toml`)
4. Global config (`~/.config/aicommit/config.toml`)
5. Hardcoded defaults

</details>

<details>
<summary><b>How It Works</b> — pipeline, splitting, retry logic</summary>

```
 ┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
 │ git add  │────>│ Read     │────>│ Split by │────>│ Generate │
 │ (auto)   │     │ staged   │     │ directory│     │ commit   │
 │          │     │ diff     │     │          │     │ message  │
 └──────────┘     └──────────┘     └──────────┘     └──────────┘
                                                           │
 ┌──────────┐     ┌──────────┐     ┌────────────────┐     │
 │   git    │<────│ Parse    │<────│ Retry on       │<────│
 │  commit  │     │ JSON/    │     │ parse failure  │     │
 │          │     │ fallback │     │ (if needed)    │     │
 └──────────┘     └──────────┘     └────────────────┘     │
```

1. **Auto-stage** (optional): `git add .` if nothing is staged.
2. **Read diff**: uses libgit2 to read the staged diff in-process.
3. **Split**: groups files by top-level directory. Small diffs pass through as one group.
4. **Generate**: builds a system prompt enforcing Conventional Commits and JSON output.
5. **Retry**: if the response isn't valid JSON, sends a stricter repair prompt.
6. **Parse**: extracts JSON (or falls back to regex) into a structured `CommitMessage`.
7. **Commit**: stages the relevant files and creates the commit.

### Grouping Example

```
src/auth/login.rs     → group "auth"
src/auth/logout.rs    → group "auth"
src/db/pool.rs        → group "db"
README.md             → group "root"
```

Each group produces exactly one Conventional Commit. If there are too many groups
(more than `max_commits`), smaller groups are merged into a `"misc"` group.

</details>

<details>
<summary><b>Benchmarks & Comparison</b></summary>

| Tool | Language | Repo size | Time | Binary size |
|------|----------|-----------|------|-------------|
| aicommits (Node) | JavaScript | 5k files | ~8s | ~5 MB |
| **aicommit (Rust)** | **Rust** | **5k files** | **~0.4s** | **~2 MB** |
| git-cz (Node) | JavaScript | 5k files | ~6s | ~30 MB |
| aicommit (w/ Ollama) | — | 5k files | ~1.2s | — |

Benchmarks on a 2022 MacBook Air (M2, 8 GB RAM). 340 lines across 12 files.
Ollama model: `qwen2.5-coder:7b`. aicommit is **40x faster** due to libgit2
(no subprocess), native Rust, and async HTTP.

| Feature | aicommits (Node) | git-cz | **aicommit (Rust)** |
|---------|-----------------|--------|---------------------|
| Speed | ~8s | ~6s | **~0.4s** |
| Privacy | Cloud only | CLI helper | **Local-first** |
| Atomic commits | No | No | **Yes** |
| Strict CC | No | Interactive | **Automatic** |
| Multi-provider | No | No | **8 providers** |
| Auto-stage | No | No | **Yes** |
| Dry-run | No | No | **Yes** |
| Interactive | No | No | **Yes** |
| Binary size | ~5 MB | ~30 MB | **~2 MB** |
| Multi-language | EN only | — | **Multi** |

</details>

<details>
<summary><b>Project Structure</b></summary>

```
src/
├── main.rs          # CLI entry point (clap)
├── config.rs        # Configuration loading and merging
├── git.rs           # git2 wrapper: diff, add, commit, status
├── interactive.rs   # Interactive commit selection UI
├── llm/
│   ├── mod.rs       # Provider enum, builder, retry logic
│   ├── ollama.rs    # Ollama provider
│   ├── openai_compat.rs  # OpenAI-compatible providers
│   ├── anthropic.rs # Anthropic provider
│   ├── gemini.rs    # Gemini provider
│   └── mock.rs      # Mock provider for testing
├── parser.rs        # LLM response parsing (JSON + text fallback)
├── prompt.rs        # System and user prompt construction
└── splitter.rs      # Atomic commit grouping logic
tests/
├── e2e_workflow.rs  # End-to-end workflow tests
└── git_integration.rs # Git integration tests
```

</details>

---

## Support

If aicommit helps you ship cleaner commits, consider giving the repo a star ⭐ — it helps others discover the project.

[![GitHub stars](https://img.shields.io/github/stars/Mohameddhiab/aicommit?style=social)](https://github.com/Mohameddhiab/aicommit/stargazers)

---

## Contributing

Bug reports, provider additions, prompt improvements, and PRs are welcome.

Check [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines. Run `cargo test && cargo clippy && cargo fmt --check` before submitting.

---

## License

MIT © [Mohamed Dhiab](https://github.com/Mohameddhiab)
