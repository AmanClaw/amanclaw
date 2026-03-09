# Contributing to AmanClaw

Thank you for your interest in contributing to AmanClaw!

## Getting Started

1. Fork and clone the repository
2. Install Rust 1.88+ via [rustup](https://rustup.rs/)
3. Copy `config.example.yaml` to `config.yaml`
4. Run tests: `cd rust && cargo test --workspace`
5. Run the bot: `cd rust && cargo run -p amanclaw-cli`

## Development Workflow

1. Create a branch from `main`
2. Make your changes
3. Ensure `cargo fmt --all` passes
4. Ensure `cargo clippy --workspace -- -D warnings` passes
5. Ensure `cargo test --workspace` passes
6. Submit a pull request

## Code Style

- Follow standard Rust conventions
- Use `anyhow::Result` for error handling in application code
- Use `thiserror` for library error types
- Use `tracing` for logging (`info!`, `warn!`, `error!`)
- Keep functions small and focused
- Write tests for new functionality

## Adding a New Skill

1. Create a new crate: `cargo new --lib rust/plugins/skill-myskill`
2. Add `amanclaw-traits` as a dependency
3. Implement the `Skill` trait:
   - `metadata()` — name, description, version
   - `parameters_schema()` — JSON Schema for tool parameters
   - `execute()` — skill logic
4. Register in `rust/crates/amanclaw-core/src/lib.rs`
5. Add to workspace members in `rust/Cargo.toml`
6. Add tests

See `rust/plugins/skill-solat/` for a complete example.

## Adding a Python Plugin

1. Create `plugins/skill_myskill.py`
2. Implement `handle(params)` function returning a JSON string
3. Add to `script_plugins` in `config.yaml`

See `plugins/skill_hadith.py` for a complete example.

## Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/):
- `feat:` new feature
- `fix:` bug fix
- `docs:` documentation
- `chore:` maintenance
- `ci:` CI/CD changes
- `refactor:` code refactoring
- `test:` adding/fixing tests

## Reporting Issues

- **Bugs:** Use the bug report template
- **Features:** Use the feature request template
- **New Skills:** Use the new skill template
- **Security:** See [SECURITY.md](SECURITY.md)
