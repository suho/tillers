# Tillers Development Guidelines

Auto-generated from all feature plans. Last updated: 2025-09-21

## Active Technologies
- **Language**: Rust 1.75+
- **Primary Dependencies**: cocoa, objc (Objective-C interop), tokio (async runtime)
- **macOS Frameworks**: Core Graphics, Accessibility APIs
- **Storage**: File-based configuration (TOML/JSON)
- **Testing**: cargo test, integration tests for window management scenarios
- **Target Platform**: macOS 12+ (Monterey and later)

## Project Structure
```
src/
├── models/          # Data models (Workspace, TilingPattern, WindowRule)
├── services/        # Business logic (WorkspaceManager, WindowManager)
├── cli/            # Command-line interface
└── lib/            # Core libraries and utilities

tests/
├── contract/       # API contract tests
├── integration/    # End-to-end workflow tests
└── unit/          # Component unit tests
```

## Commands
### Rust Development
```bash
# Build project
cargo build

# Run tests
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy

# Run with debugging
RUST_LOG=debug cargo run
```

### macOS Integration Testing
```bash
# Test accessibility permissions
cargo test accessibility_permissions

# Test window management APIs
cargo test window_apis

# Performance benchmarks
cargo bench
```

## Code Style
### Rust Guidelines
- Use `rustfmt` for consistent formatting
- Enable `clippy` lints for code quality
- Prefer `Result<T, E>` over panics for error handling
- Use `async/await` for I/O operations with tokio
- Document public APIs with rustdoc comments
- Follow Rust naming conventions (snake_case for functions, PascalCase for types)

### macOS Interop
- Use `cocoa` crate for Objective-C runtime interaction
- Wrap all macOS API calls in proper error handling
- Test permissions before attempting privileged operations
- Use `NSString` conversions for string handling with Objective-C

## Recent Changes
### Feature 001: TilleRS Core Application (2025-09-21)
- Added keyboard-first tiling window manager for macOS
- Implemented workspace management with logical groupings
- Created multi-monitor support with consistent layouts
- Added Rust + Objective-C interop architecture
- Performance targets: <200ms workspace switching, <100MB memory

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->