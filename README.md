# TilleRS

A keyboard-first tiling window manager for macOS built in Rust.

## Overview

TilleRS helps macOS power users stay in flow by organizing windows into logical workspaces and providing instant, keyboard-driven context switching. The current codebase focuses on building the core workspace and keyboard services, establishing guardrails for macOS integration, and collecting the metrics needed to keep workspace switches under 200 ms.

## Status

- Pre-alpha demonstration build; window manipulation and most macOS integration points are still stubs
- Asynchronous workspace management with event hooks, validation, and performance metrics is implemented
- Keyboard shortcut handling now enforces the Option (Alt/Option) modifier by default and migrates legacy Command shortcuts automatically
- CLI tooling, UI integrations, and on-device window management are in active development

## Current Capabilities

- Create and manage workspaces with validation, event emission, and metrics tracking via `WorkspaceManager`
- Maintain keyboard shortcut mappings with conflict detection and Option-key enforcement through `KeyboardHandler`
- Rich domain models for tiling patterns, window rules, monitor configurations, and application profiles
- Structured tracing instrumentation for diagnostics (`tillers=debug,info`)

## Roadmap

- Integrate with macOS Accessibility APIs for real window placement and focus management
- Expose workspace operations through a command-line interface and user-facing UI layer
- Flesh out contract and integration tests in `tests/integration_tests.rs` and Criterion benchmarks in `benches/`
- Persist configuration to `~/.config/tillers/` and support live reload of workspace layouts

## Requirements

- macOS 12.0 (Monterey) or later
- Accessibility permissions for window management
- Input monitoring permissions for global shortcuts

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/tillers/tillers
cd tillers

# Build the project
cargo build --release

# Install binary
cargo install --path .
```

## Usage

### Run the Demo

The current binary boots the async services, creates a default workspace, and logs activity. Run it with tracing enabled to inspect the flow:

```bash
RUST_LOG=tillers=debug,info cargo run
```

Watch the log output for workspace creation, activation, and keyboard shortcut migration events. CLI subcommands such as `tillers workspace list` are not implemented yet and are tracked on the roadmap.

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Option + 1-9` | Switch to workspace 1-9 |
| `Option + Shift + 1-9` | Move window to workspace 1-9 |
| `Option + Space` | Cycle through tiling layouts |
| `Option + Enter` | Focus next window |

> Legacy Command-based shortcuts are automatically migrated to use Option when loaded into the keyboard handler.

## Configuration

TilleRS will use TOML configuration files located at `~/.config/tillers/`:

```toml
# ~/.config/tillers/config.toml
[workspace]
default_layout = "main-stack"
auto_balance = true

[keybindings]
workspace_switch = "option"
window_move = "option+shift"

[applications.terminal]
workspace = "development"
layout = "columns"
```

## Development

### Prerequisites

- Rust 1.75 or later
- Xcode Command Line Tools

### Building

```bash
# Development build
cargo build

# Release build with optimizations
cargo build --release

# Run tests (many integration tests intentionally panic until features land)
cargo test

# Run with debug logging
RUST_LOG=debug cargo run
```

### Testing & Benchmarks

```bash
# Run all tests
cargo test

# Run integration test bundle (expected to fail until contract work lands)
cargo test --test integration_tests

# Build and run performance benchmarks
cargo bench
```

### Code Quality

```bash
# Format code
cargo fmt --all

# Run linter
cargo clippy -- -D warnings
```

## Architecture

```
src/
├── config/          # Configuration loading and defaults
├── lib.rs           # Crate exports and error types
├── macos/           # macOS integration scaffolding
├── main.rs          # Demo entry point wiring services together
├── models/          # Domain models (Workspace, TilingPattern, KeyboardMapping, ...)
├── services/        # Async services (WorkspaceManager, KeyboardHandler, TilingEngine, ...)
├── ui/              # Placeholder for future UI integrations
└── ...

benches/             # Criterion benchmarks (workspace switching, window positioning)
tests/               # Contract and integration tests (currently red by design)
resources/           # macOS bundle metadata (Info.plist, entitlements)
```

### Key Components

- **WorkspaceManager**: Handles workspace creation, switching, validation, and emits events/metrics
- **KeyboardHandler**: Registers shortcuts, enforces Option modifiers, and surfaces shortcut events
- **TilingEngine**: Calculates window layouts (implementation in progress)
- **macos module**: Provides the bridge into macOS Accessibility APIs (currently scaffolding)

## Performance Targets

- Workspace switching under 200 ms (95th percentile)
- Window positioning under 50 ms
- Memory usage below 100 MB during normal operation
- Idle CPU utilization below 1%

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes following the coding standards
4. Add tests for new functionality (or mark TODOs when features are still in flight)
5. Run the test suite (`cargo test`)
6. Commit your changes (`git commit -m 'Add amazing feature'`)
7. Push to the branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

### Development Guidelines

- Follow Rust naming conventions
- Use `cargo fmt --all` before committing
- Ensure `cargo clippy -- -D warnings` passes
- Document public APIs and keep specs/contracts in sync
- Test on multiple macOS versions when possible

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Support

- **Issues**: [GitHub Issues](https://github.com/tillers/tillers/issues)
- **Discussions**: [GitHub Discussions](https://github.com/tillers/tillers/discussions)
- **Documentation**: [Wiki](https://github.com/tillers/tillers/wiki)

## Acknowledgments

- Built with [Rust](https://www.rust-lang.org/)
- Uses [cocoa-rs](https://github.com/servo/core-foundation-rs) for macOS integration
- Inspired by [yabai](https://github.com/koekeishiya/yabai) and [Amethyst](https://github.com/ianyh/Amethyst)
