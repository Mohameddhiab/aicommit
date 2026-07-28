<p align="center">
  <img src="media/banner.svg" width="800" alt="git-doctor banner">
</p>

<div align="center">

[![CI](https://github.com/Mohameddhiab/aicommit/actions/workflows/ci.yml/badge.svg)](https://github.com/Mohameddhiab/aicommit/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Rust](https://img.shields.io/badge/rust-2021-orange.svg)

</div>

**Git History Doctor — diagnose, fix, and maintain clean git history.**

> **diagnose → plan → apply → verify**

---

## Why git-doctor?

- **Analyze** — score commits on message quality, atomicity, size, and convention. Detect WIP, vague, oversized, and mixed-concern commits.
- **Plan** — generates a structured plan with operations (Squash, Reword, Split) to fix issues.
- **Apply** — safely applies the plan with automatic backup branches, dry-run mode, and remote detection.
- **Check** — pre-push hook blocks WIP/vague commits before they reach remote.
- **Commit** — generate well-structured Conventional Commits from staged changes (8 providers).

```bash
# Before: vague history full of "fix", "wip", "stuff"
doctor analyze          # score your last 10 commits
doctor plan             # generate a cleanup plan
doctor apply plan.json  # safe apply with backup
```

---

## Quick Start

```bash
cargo install git-doctor
cd your-repo
doctor analyze                     # check health of last 10 commits
doctor commit                       # generate atomic commit from staged changes
```

---

## Features

### History Doctor
| Command | Description |
|---------|-------------|
| `doctor analyze` | Score commits on quality, atomicity, size. Detect WIP/vague/mixed. Output text, JSON, or HTML. |
| `doctor plan` | Generate a cleanup plan with Squash, Reword, Split operations. |
| `doctor apply plan.json` | Safe apply with backup branch, dry-run, force flag, remote detection. |
| `doctor check --pre-push` | Pre-push hook that blocks low-quality commits. |
| `doctor init` | Install hooks and create default config. |
| `doctor undo` | Undo the last commit (`git reset --soft HEAD~1`). |

### Commit Generator
| Feature | Description |
|---------|-------------|
| Atomic commits | Groups files by directory, one Conventional Commit per group |
| Interactive mode (`-i`) | Pick which groups to commit, edit messages |
| Auto-stage | `doctor commit` = `git add .` + commit; disable with `--no-stage` |
| Dry-run (`-n`) | Preview generated messages without committing |
| Multi-language | Generate messages in French, Spanish, German, etc. (`--lang fr`) |
| 8 providers | Ollama, OpenAI, Anthropic, Groq, DeepSeek, Mistral, Gemini, OpenRouter |
| Retry + backoff | 3 attempts on timeouts and server errors |

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

> **Auto-detection:** If no provider is specified, doctor checks for a running Ollama instance first, then falls back to any provider with a configured API key.

---

## Installation

```bash
cargo install git-doctor                           # from source
cargo install --git https://github.com/Mohameddhiab/aicommit.git  # latest
```

Pre-built binaries for Linux (x86_64), macOS (x86_64 + aarch64), and Windows (x86_64) on the [Releases page](https://github.com/Mohameddhiab/aicommit/releases).

---

## Usage

```bash
doctor                          # analyze, plan, apply, check, commit
doctor analyze --commits 20     # analyze 20 commits
doctor analyze --format html --open  # open HTML report in browser
doctor plan --output plan.json  # save plan to file
doctor apply plan.json          # safe apply with backup
doctor apply plan.json --dry-run    # preview only
doctor apply plan.json --force      # force even if pushed
doctor check --pre-push         # run as pre-push hook
doctor commit                   # generate atomic commit
doctor commit --dry-run         # preview only
doctor commit --interactive     # pick groups + edit messages
doctor commit --single          # one commit, no splitting
doctor commit --lang fr         # messages in French
doctor config --provider openai --api-key sk-...
doctor undo                     # undo last commit
doctor init                     # install hooks + config
doctor uninstall                # remove hooks
```

<details>
<summary><b>doctor analyze scoring</b></summary>

Each commit is scored on four dimensions (0–100):

| Dimension | Description |
|-----------|-------------|
| **Message Quality** | Subject length, body presence, conventional commit format |
| **Atomicity** | Single concern? Files belong to one domain? |
| **Size** | Reasonable diff size (not too large, not too small) |
| **Convention** | Proper formatting, no WIP/vague patterns |

Flags:
- ⚠ WIP — subject contains "wip", "tmp", "fixme", etc.
- ⚠ Vague — subject is too generic ("fix", "update", "stuff")
- ⚠ Oversized — touches too many files or lines
- ⚠ Mixed Concern — files span multiple domains

</details>

<details>
<summary><b>Configuration</b> — config file, env vars, resolution order</summary>

### Config File

**Project config** — `.doctor.toml` in your project root:
```toml
[provider]
default = "ollama"
```

**Global config** — `~/.config/doctor/config.toml`:
```toml
[openai]
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
3. Project config (`./.doctor.toml`)
4. Global config (`~/.config/doctor/config.toml`)
5. Hardcoded defaults

</details>

<details>
<summary><b>Project Structure</b></summary>

```
src/
├── main.rs          # CLI entry point (clap)
├── analyze.rs       # History scoring and quality analysis
├── apply.rs         # Safe plan application with backup
├── config.rs        # Configuration loading and merging
├── display.rs       # Boxed terminal output
├── git.rs           # git2 wrapper: diff, add, commit, walk history
├── hook.rs          # Pre-push hook management
├── interactive.rs   # Interactive commit selection UI
├── llm/             # AI providers
├── parser.rs        # LLM response parsing
├── plan.rs          # Cleanup plan generation
├── prompt.rs        # Prompt construction
├── report.rs        # Report output (text, JSON, HTML)
├── splitter.rs      # Atomic commit grouping logic
tests/
├── e2e_workflow.rs  # End-to-end workflow tests
└── git_integration.rs # Git integration tests
```

</details>

---

## Support

If git-doctor helps you ship cleaner repositories, consider giving the repo a star ⭐.

[![GitHub stars](https://img.shields.io/github/stars/Mohameddhiab/aicommit?style=social)](https://github.com/Mohameddhiab/aicommit/stargazers)

---

## Contributing

Bug reports, provider additions, and PRs are welcome.

Check [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines. Run `cargo test && cargo clippy -- -D warnings && cargo fmt --check` before submitting.

---

## License

MIT © [Mohamed Dhiab](https://github.com/Mohameddhiab)
