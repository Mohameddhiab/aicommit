# Contributing

Thanks for your interest in aicommit! Here's how to get started.

## Prerequisites

- Rust 1.81+
- Windows: MSVC Build Tools (Visual Studio 2022 Build Tools with "Desktop development with C++")

## Setup

```bash
git clone https://github.com/Mohameddhiab/aicommit.git
cd aicommit
cargo build
```

## Development Workflow

1. **Find or create an issue** — check [open issues](https://github.com/Mohameddhiab/aicommit/issues) first.
2. **Fork the repo** and create a feature branch.
3. **Make your changes** — keep them focused on one issue.
4. **Run checks** before committing:

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

5. **Open a PR** with a clear description of what you changed and why.

## Project Structure

```
src/
├── main.rs          # CLI entry point (clap)
├── config.rs        # Configuration loading and merging
├── git.rs           # git2 wrapper: diff, add, commit, status
├── interactive.rs   # Interactive commit selection UI
├── llm/             # AI providers (ollama, openai, anthropic, gemini, mock)
├── parser.rs        # LLM response parsing (JSON + text fallback)
├── prompt.rs        # System and user prompt construction
├── splitter.rs      # Atomic commit grouping logic
└── display.rs       # Boxed terminal output
tests/
├── e2e_workflow.rs  # End-to-end workflow tests
└── git_integration.rs # Git integration tests
```

## Adding a Provider

1. Create `src/llm/<name>.rs` with `chat()` and `list_models()` methods.
2. Add a variant to `AnyProvider` in `src/llm/mod.rs`.
3. Add the provider to `ProviderKind` in `src/config.rs`.
4. Wire it in `build_provider()` and `handle_config_command()`.

## Code Style

- Follow existing patterns (error handling with `anyhow`, async with `tokio`).
- No `unsafe` code (enforced by `#![deny(unsafe_code)]`).
- No unwrap in production code — use `?` or `context()`.
- Add tests for new functionality.

## License

MIT — see [LICENSE](LICENSE).
