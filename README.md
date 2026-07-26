# aicommit

> Generate perfect, atomic Git commits using AI — fast, local, reliable.

[![CI](https://github.com/Mohameddhiab/aicommit/actions/workflows/ci.yml/badge.svg)](https://github.com/Mohameddhiab/aicommit/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/Mohameddhiab/aicommit/graph/badge.svg?token=YOUR_CODECOV_TOKEN)](https://codecov.io/gh/Mohameddhiab/aicommit)
[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

`aicommit` stages your changes, reads the diff, calls an AI provider, and writes clean
Conventional Commit messages for you — in **one command**, in **under a second**.

## Why?

Writing good commit messages is tedious. Most developers fall back to `wip`
and `fix bug`, which destroys the project history and confuses code reviewers
and LLMs alike. `aicommit` does the work for you:

```bash
aicommit
# ▶ aicommit — 340 lines to analyze
# ▸ Group 'auth' — 200 lines to analyze
# ✔ Committed: feat(auth): implement JWT token expiration
# ▸ Group 'db' — 140 lines to analyze
# ✔ Committed: refactor(db): remove unused Redis client pool
```

## Features

- **Atomic commits** — splits large diffs into logical commits, one per domain
  (`src/auth/`, `src/db/`, `…`).
- **Local-first** — uses Ollama by default, zero code sent to the cloud.
- **Multi-provider** — Ollama, OpenAI, Anthropic, Groq, DeepSeek, Mistral,
  Gemini, OpenRouter.
- **Interactive mode** — when multiple groups are detected, pick which to commit and edit messages.
- **Dry-run** — preview generated messages without committing.
- **Strict Conventional Commits** — `feat`, `fix`, `docs`, `refactor`, …
- **Fast** — native Rust binary, async HTTP, ~0.4s on a 5k-file repo.

## Install

```bash
# Cargo (recommended)
cargo install aicommit

# Homebrew
brew install Mohameddhiab/tap/aicommit
```

## Usage

```bash
# Default: auto-stages changes, reads diff, splits into atomic commits
aicommit

# Generate message(s) without committing
aicommit --dry-run

# Interactive mode — select which atomic commits to apply
aicommit --interactive

# Force a single commit (skip atomic splitting)
aicommit --single

# Disable auto-staging (you must `git add` first)
aicommit --no-stage

# Pick a provider and model
aicommit --provider ollama   --model qwen2.5-coder:7b
aicommit --provider openai   --model gpt-4o
aicommit --provider groq     --model llama-3.3-70b-versatile
aicommit --provider anthropic --model claude-3-5-sonnet-20241022
aicommit --provider gemini   --model gemini-2.0-flash
aicommit --provider deepseek --model deepseek-coder
aicommit --provider mistral  --model codestral-latest
aicommit --provider openrouter --model anthropic/claude-3.5-sonnet

# Generate messages in French
aicommit --lang fr

# Configure an API key (stored in ~/.config/aicommit/config.toml)
aicommit config --provider openai --api-key sk-...
```

## Configuration

`aicommit` reads configuration from (highest priority first):

1. CLI flags (`--provider`, `--model`, `--lang`, `--api-key`).
2. Environment variables (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, …).
3. Project config: `./.aicommit.toml`.
4. Global config: `~/.config/aicommit/config.toml` (or `~/.aicommit.toml`).

See [`.aicommit.toml`](./.aicommit.toml) for a complete example.

## Benchmark

| Tool              | Repo size | Time   | Binary size |
|-------------------|-----------|--------|-------------|
| aicommits (Node)  | 5k files  | ~8s    | ~5 MB       |
| aicommit (Rust)   | 5k files  | ~0.4s  | ~2 MB       |

## Status

✅ **Stable** — Ollama, OpenAI-compatible providers, Anthropic, Gemini, multi-commit splitting, interactive menu, dry-run, auto-stage, retry on parse failure, colored output, CI/CD, publish-ready.

## License

MIT
