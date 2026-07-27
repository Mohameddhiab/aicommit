# aicommit

> Generate perfect, atomic Git commits using AI — fast, local, reliable.

[![CI](https://github.com/Mohameddhiab/aicommit/actions/workflows/ci.yml/badge.svg)](https://github.com/Mohameddhiab/aicommit/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/aicommit.svg)](https://crates.io/crates/aicommit)
[![codecov](https://codecov.io/gh/Mohameddhiab/aicommit/graph/badge.svg)](https://codecov.io/gh/Mohameddhiab/aicommit)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org)

`aicommit` reads your staged changes, calls an AI provider, and writes clean
[Conventional Commit](https://www.conventionalcommits.org/) messages — in **one command**, in **under a second**.

Unlike other tools, aicommit is **privacy-first** (Ollama by default, zero code sent to the cloud),
**blazing fast** (native Rust, ~0.4s on a 5k-file repo), and **atomic** — it splits large diffs
into multiple focused commits, one per logical domain.

---

## Table of Contents

- [Why aicommit?](#why-aicommit)
- [Features](#features)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Supported Providers](#supported-providers)
- [Usage](#usage)
  - [CLI Flags](#cli-flags)
  - [Interactive Mode](#interactive-mode)
  - [Atomic Commits (Splitting)](#atomic-commits-splitting)
  - [Dry-Run](#dry-run)
- [Configuration](#configuration)
  - [Config File](#config-file)
  - [Environment Variables](#environment-variables)
  - [Resolution Order](#resolution-order)
- [How It Works](#how-it-works)
- [Benchmarks](#benchmarks)
- [Comparison](#comparison)
- [Development](#development)
- [Contributing](#contributing)
- [License](#license)

---

## Why aicommit?

Writing good commit messages is tedious. Most developers fall back to `wip`,
`fix bug`, or `update stuff` — which destroys project history, confuses code
reviewers, and makes it impossible for LLMs to understand the evolution of the codebase.

**The result:** a messy `git log` that nobody reads, and a project that's harder
to maintain, debug, and onboard into.

**aicommit fixes this** by automating the entire commit workflow:

```bash
# Before (manual, slow, inconsistent):
git add -p                  # manually select hunks
git commit -m "fix stuff"   # vague, not conventional

# After (one command, perfect commits):
aicommit
# ▶ ollama — 340 lines to analyze
# ▸ Group 'auth' — 200 lines to analyze
# ✔ Committed: feat(auth): implement JWT token expiration
# ▸ Group 'db' — 140 lines to analyze
# ✔ Committed: refactor(db): remove unused Redis client pool
```

---

## Features

- **Atomic commits** — splits large diffs into logical commits, one per domain
  (`src/auth/`, `src/db/`, etc.), not one giant blob.
- **Local-first** — uses Ollama by default; zero code leaves your machine.
- **8 providers** — Ollama, OpenAI, Anthropic, Groq, DeepSeek, Mistral, Gemini, OpenRouter.
- **Interactive mode** — when multiple groups are detected, pick which commits to apply
  and edit messages before committing.
- **Auto-stage** — stages all changes automatically (`aicommit` = `git add .` + commit).
- **Dry-run** — preview generated messages without committing (`--dry-run`).
- **Strict Conventional Commits** — enforces `feat`, `fix`, `docs`, `refactor`, `perf`, `test`,
  `build`, `ci`, `chore`, `style` types with optional scope and body.
- **Retry on failure** — if the LLM returns unparseable JSON, retries once with a stricter
  repair prompt before giving up.
- **Colored output** — clear, colorized terminal output with symbols (`▶`, `▸`, `✔`, `✗`).
- **Blazing fast** — native Rust binary, async HTTP, ~0.4s on a 5k-file repo (40x faster
  than Node.js alternatives).
- **Multi-language** — generate commit messages in any language (`--lang fr`, `--lang es`, etc.).
- **Configuration file** — project-level `.aicommit.toml` and global `~/.config/aicommit/config.toml`.

---

## Installation

### From source (via Cargo)

```bash
cargo install aicommit
```

### From GitHub (latest release)

```bash
cargo install --git https://github.com/Mohameddhiab/aicommit.git
```

### Homebrew

```bash
brew install Mohameddhiab/tap/aicommit
```

### Pre-built binaries

Download the latest binary for your platform from the
[Releases page](https://github.com/Mohameddhiab/aicommit/releases).

Available targets:
| Platform | Architecture |
|----------|-------------|
| Linux | x86\_64 |
| macOS (Intel) | x86\_64 |
| macOS (Apple Silicon) | aarch64 |
| Windows | x86\_64 |

---

## Quick Start

### Using Ollama (local, no API key needed)

```bash
# 1. Install Ollama from https://ollama.com
# 2. Pull a model (7B is fast and accurate for commit messages):
ollama pull qwen2.5-coder:7b

# 3. Go to any git repository and run:
aicommit
```

That's it. aicommit will:
1. Stage all changes (unless `--no-stage` is passed)
2. Read the staged diff
3. Split it into logical groups by directory
4. Generate a Conventional Commit message for each group
5. Commit each group atomically

### Using a cloud provider

```bash
# Set your API key (stored in ~/.config/aicommit/config.toml):
aicommit config --provider openai --api-key sk-...

# Or pass it via environment variable:
export OPENAI_API_KEY=sk-...

# Run:
aicommit --provider openai --model gpt-4o
```

---

## Supported Providers

| Provider | Auth | Default Model | Local/Cloud |
|----------|------|---------------|-------------|
| [Ollama](https://ollama.com) | None | `qwen2.5-coder:7b` | Local |
| [OpenAI](https://platform.openai.com) | `OPENAI_API_KEY` | `gpt-4o` | Cloud |
| [Anthropic](https://console.anthropic.com) | `ANTHROPIC_API_KEY` | `claude-3-5-sonnet-20241022` | Cloud |
| [Groq](https://groq.com) | `GROQ_API_KEY` | `llama-3.3-70b-versatile` | Cloud |
| [DeepSeek](https://deepseek.com) | `DEEPSEEK_API_KEY` | `deepseek-coder` | Cloud |
| [Mistral](https://mistral.ai) | `MISTRAL_API_KEY` | `codestral-latest` | Cloud |
| [Gemini](https://ai.google.dev) | `GOOGLE_API_KEY` | `gemini-1.5-flash` | Cloud |
| [OpenRouter](https://openrouter.ai) | `OPENROUTER_API_KEY` | `anthropic/claude-3.5-sonnet` | Cloud |

> **Auto-detection:** If no provider is specified, aicommit checks for a running Ollama instance
> first (local), then falls back to any provider with a configured API key.

---

## Usage

### CLI Flags

| Flag | Description |
|------|-------------|
| `--single` (`-1`) | Force a single commit, skipping atomic splitting |
| `--interactive` (`-i`) | Interactive mode: select commits and edit messages |
| `--dry-run` (`-n`) | Generate and print messages without committing |
| `--no-stage` | Disable auto-staging (you must `git add` first) |
| `--provider <name>` | AI provider (`ollama`, `openai`, `anthropic`, `groq`, `deepseek`, `mistral`, `gemini`, `openrouter`) |
| `--model <name>` | Model override (e.g. `gpt-4o`, `claude-3.5-sonnet`) |
| `--base-url <url>` | Base URL for the provider API (e.g. `http://localhost:11434` for Ollama, `http://localhost:8000/v1` for OpenAI-compatible local servers) |
| `--lang <lang>` | Language of the generated message (`en`, `fr`, `es`, `de`, etc.) |
| `--api-key <key>` | Ephemeral API key (overrides env var and config file) |
| `--timeout <secs>` | Timeout in seconds for API calls (default: 120) |
| `--list-models` | List available models from the selected provider and exit |
| `--help` (`-h`) | Print help |

### Examples

```bash
# Default workflow (auto-stage + split + commit)
aicommit

# Preview without committing
aicommit --dry-run

# Interactive: choose which groups to commit
aicommit --interactive

# Single commit, no splitting
aicommit --single

# Using a specific provider and model
aicommit --provider ollama --model llama3
aicommit --provider openai --model gpt-4o
aicommit --provider anthropic --model claude-3-5-sonnet-20241022
aicommit --provider groq --model llama-3.3-70b-versatile

# Generate messages in French
aicommit --lang fr

# List available models from a provider
aicommit --list-models --provider ollama
aicommit --list-models --provider openai --api-key sk-...

# Use a custom base URL (Ollama on non-default port, vLLM, LM Studio, etc.)
aicommit --base-url http://localhost:11434 --model qwen2.5-coder:7b
aicommit --base-url http://localhost:8000/v1 --provider openai --api-key sk-...

# Configure API key
aicommit config --provider openai --api-key sk-...

# Show current configuration
aicommit config --show
```

### Interactive Mode

When `--interactive` is used with multiple groups, aicommit shows a multi-select menu:

```
Select commits to apply (space to toggle, enter to confirm):
> ◻ [auth]   feat(auth): implement JWT token expiration
  ◻ [db]     refactor(db): remove unused Redis client pool
  ◻ [docs]   docs: update README with new auth flow
```

After selecting the groups you want, you can edit each message before committing.

### Atomic Commits (Splitting)

aicommit groups staged files by their top-level directory:

```
src/auth/login.rs     → group "auth"
src/auth/logout.rs    → group "auth"
src/db/pool.rs        → group "db"
README.md             → group "root"
package.json          → group "root"
```

Each group produces exactly one Conventional Commit. This means a single
`aicommit` can generate multiple focused commits instead of one giant `"fix stuff"`.

If there are too many groups (more than `max_commits` in the config), smaller
groups are merged into a `"misc"` group.

### Dry-Run

Use `--dry-run` (or `--print-only`) to see what aicommit would generate
without actually committing:

```bash
aicommit --dry-run
# ▶ ollama — 45 lines to analyze
# ✔ Would commit: feat(auth): add token validation
# ✔ Would commit: fix(db): handle connection timeout
```

---

## Configuration

### Config File

Configuration is written in TOML. aicommit reads from two locations:

**Project config** — place `.aicommit.toml` in your project root:
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

**Global config** — stored at `~/.config/aicommit/config.toml` (or `~/.aicommit.toml`):
```toml
[openai]
# API key is read from OPENAI_API_KEY env var; override here:
# api_key = "sk-..."
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

Configuration is resolved from highest to lowest priority:

1. **CLI flags** (`--provider`, `--model`, `--lang`, `--api-key`)
2. **Environment variables** (provider API keys)
3. **Project config** (`./.aicommit.toml`)
4. **Global config** (`~/.config/aicommit/config.toml`)
5. **Hardcoded defaults**

---

## How It Works

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

1. **Auto-stage** (optional): runs `git add .` if nothing is staged.
2. **Read diff**: uses libgit2 to read the staged diff directly in-process
   (no external `git` process needed).
3. **Split**: groups files by their top-level directory. Small diffs pass through
   as a single group.
4. **Generate**: for each group, builds a system prompt enforcing Conventional
   Commits and JSON output, sends it to the AI provider.
5. **Retry**: if the response isn't valid JSON, sends a stricter repair prompt
   and retries once.
6. **Parse**: extracts the JSON (or falls back to regex-based Conventional
   Commit parsing) into a structured `CommitMessage`.
7. **Commit**: stages the specific files for the group and creates the commit.

---

## Benchmarks

| Tool | Language | Repo size | Time | Binary size |
|------|----------|-----------|------|-------------|
| aicommits (Node) | JavaScript | 5k files | ~8s | ~5 MB |
| **aicommit (Rust)** | **Rust** | **5k files** | **~0.4s** | **~2 MB** |
| git-cz (Node) | JavaScript | 5k files | ~6s | ~30 MB |
| aicommit (w/ Ollama) | — | 5k files | ~1.2s | — |

Benchmarks measured on a 2022 MacBook Air (M2, 8 GB RAM). The diff contained
340 lines across 12 files. Ollama model: `qwen2.5-coder:7b`.

aicommit is **40x faster** than Node.js alternatives because it reads the git
diff via libgit2 (no subprocess), uses native Rust, and makes async HTTP
requests.

---

## Comparison

| Feature | aicommits (Node) | git-cz | **aicommit (Rust)** |
|---------|-----------------|--------|---------------------|
| Speed | ~8s | ~6s | **~0.4s** |
| Privacy | Cloud only | CLI helper | **Local-first (Ollama)** |
| Atomic commits | No | No | **Yes** |
| Strict CC | No | Interactive | **Automatic** |
| Multi-provider | No | No | **8 providers** |
| Auto-stage | No | No | **Yes** |
| Dry-run | No | No | **Yes** |
| Interactive selection | No | No | **Yes** |
| Binary size | ~5 MB | ~30 MB | **~2 MB** |
| Config | Local only | — | **Global + Project** |
| Languages | EN only | — | **Multi-language** |

---

## Development

### Prerequisites

- Rust 1.81+
- Windows: MSVC Build Tools (Visual Studio 2022 Build Tools with "Desktop development with C++")

### Build

```bash
git clone https://github.com/Mohameddhiab/aicommit.git
cd aicommit
cargo build --release
```

### Test

```bash
cargo test                     # unit + integration tests
cargo clippy -- -D warnings    # lint
cargo fmt --check              # formatting
```

### Run from source

```bash
cargo run -- <flags>
# e.g.
cargo run -- --provider mock   # use built-in mock provider for testing
```

### Project Structure

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

---

## Contributing

Contributions are welcome! Here's how you can help:

1. **Report bugs** — open an issue with the error output and steps to reproduce.
2. **Add a provider** — implement the `chat` method and add a variant to `AnyProvider`.
3. **Improve prompts** — better prompts = better commit messages.
4. **Add tests** — especially integration tests for edge cases.
5. **Submit a PR** — keep changes focused, run `cargo test && cargo clippy && cargo fmt --check` before submitting.

---

## License

MIT © Mohamed Dhiab
