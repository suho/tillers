# Repository Guidelines

## Project Structure & Module Organization
- `src/`: Rust sources organized by domain
  - `models/` (Workspace, TilingPattern, WindowRule, …)
  - `services/` (WorkspaceManager, future WindowManager, …)
  - `macos/` (platform integration), `config/`, `cli/`, `main.rs`
- `tests/`: integration and contract tests (`integration_tests.rs`, `contract/`)
- `benches/`: Criterion benchmarks (`workspace_switching.rs`, `window_positioning.rs`)
- `resources/`: macOS `Info.plist`, `entitlements.plist`
- Top-level: `Cargo.toml`, `build.rs`, tooling configs (`rustfmt.toml`, `clippy.toml`)

## Build, Test, and Development Commands
- Build: `cargo build` (use `--release` for optimized builds)
- Run locally: `RUST_LOG=debug cargo run`
- Format: `cargo fmt --all` (CI enforces `-- --check`)
- Lint: `cargo clippy -- -D warnings`
- Tests: `cargo test` (async via `#[tokio::test]` is supported)
  - Specific: `cargo test --test integration_tests`
- Benchmarks: `cargo bench` (CI uses `cargo bench --no-run`)

## Coding Style & Naming Conventions
- Rust 2021, 4 spaces, `max_width = 100` (see `rustfmt.toml`). Always run `cargo fmt`.
- Prefer idiomatic Rust: early returns, `?`, field init shorthand; avoid `unwrap()` in non-test paths.
- Naming: `snake_case` for modules/functions, `UpperCamelCase` for types/traits, `SCREAMING_SNAKE_CASE` for consts.
- Errors: use `anyhow::Result` for app flows and `thiserror` enums for domain errors (`TilleRSError`).
- Logging: use `tracing` with `env-filter` (`tillers=debug,info`).

## Testing Guidelines
- Unit tests live beside code (e.g., `src/models/workspace.rs`), integration/contract tests under `tests/`.
- Write tests for all public APIs; mirror scenarios in `tests/integration_tests.rs` and `tests/contract/*`.
- Keep contract tests failing until the feature exists; remove `panic!` guards only when implemented.
- Use `tempfile` for filesystem, avoid touching real user config.

## Commit & Pull Request Guidelines
- Commits: concise, imperative (e.g., "Add workspace switching metrics"). Optional scope prefix (e.g., `services:`).
- PRs must: describe changes and rationale, link issues, include test coverage, and note macOS version tested.
- CI must pass: format, clippy (no warnings), tests, and benchmark build.
- Small, focused PRs over broad refactors.

## Security & Configuration Tips
- macOS permissions: Accessibility and Input Monitoring are required at runtime.
- Keep `resources/Info.plist` and `entitlements.plist` consistent; do not weaken entitlements.
- User config lives under `~/.config/tillers/`; never hardcode user paths.

## Agent-Specific Instructions
- Follow this guide for edits; keep patches minimal and localized.
- Update docs when changing public APIs or CLI behavior.
- Codex agents may invoke commands located under `.codex/commands/*` without additional approval.
