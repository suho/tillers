# TilleRS

A keyboard-first tiling window manager for macOS built in Rust.

## Overview

TilleRS is a powerful tiling window manager designed specifically for macOS that emphasizes keyboard-driven workflow and productivity. It provides workspace management, multi-monitor support, and customizable window layouts while maintaining native macOS integration.

## Features

- **Keyboard-First Design**: Navigate and manage windows without touching the mouse
- **Workspace Management**: Organize applications into logical workspaces
- **Multi-Monitor Support**: Consistent layouts across multiple displays
- **Application-Specific Rules**: Customize behavior for specific applications
- **Performance Optimized**: Sub-200ms workspace switching, minimal memory footprint
- **Native macOS Integration**: Uses Accessibility APIs and Core Graphics

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

### First Run

1. Grant accessibility permissions when prompted
2. Grant input monitoring permissions for keyboard shortcuts
3. Start TilleRS: `tillers`

### Basic Commands

```bash
# Start TilleRS daemon
tillers

# List available workspaces
tillers workspace list

# Create a new workspace
tillers workspace create development

# Switch to workspace
tillers workspace switch development

# Show help
tillers --help
```

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd + 1-9` | Switch to workspace 1-9 |
| `Cmd + Shift + 1-9` | Move window to workspace 1-9 |
| `Cmd + Space` | Cycle through tiling layouts |
| `Cmd + Enter` | Focus next window |

## Configuration

TilleRS uses TOML configuration files located at `~/.config/tillers/`:

```toml
# ~/.config/tillers/config.toml
[workspace]
default_layout = "main-stack"
auto_balance = true

[keybindings]
workspace_switch = "cmd"
window_move = "cmd+shift"

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

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run
```

### Testing

```bash
# Run all tests
cargo test

# Run integration tests
cargo test --test integration_tests

# Run performance benchmarks
cargo bench
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Check for issues
cargo clippy -- -D warnings
```

## Architecture

TilleRS is built with a modular architecture:

```
src/
├── models/          # Data models (Workspace, TilingPattern, WindowRule)
├── services/        # Business logic (WorkspaceManager, WindowManager)
├── macos/          # macOS system integration
├── config/         # Configuration management
├── cli/            # Command-line interface
└── main.rs         # Application entry point
```

### Key Components

- **WorkspaceManager**: Handles workspace creation, switching, and persistence
- **WindowManager**: Interfaces with macOS Accessibility APIs for window control
- **TilingEngine**: Calculates window layouts and positions
- **KeyboardHandler**: Manages global keyboard shortcuts

## Performance

TilleRS is optimized for performance:

- **Workspace Switching**: < 200ms
- **Window Positioning**: < 50ms
- **Memory Usage**: < 100MB
- **CPU Usage**: < 1% idle

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes following the coding standards
4. Add tests for new functionality
5. Run the test suite (`cargo test`)
6. Commit your changes (`git commit -m 'Add amazing feature'`)
7. Push to the branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

### Development Guidelines

- Follow Rust naming conventions
- Use `rustfmt` for code formatting
- Ensure all tests pass before submitting
- Add documentation for public APIs
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