# Research: TilleRS Technical Decisions

## macOS Window Management APIs

### Decision: Use Accessibility APIs with Core Graphics fallback
**Rationale**: Accessibility APIs (AXUIElement) provide the most reliable and Apple-approved method for window manipulation on macOS. Core Graphics APIs offer additional capabilities for window enumeration and monitoring.

**Alternatives considered**:
- Private APIs only (CGSSpace, etc.) - Rejected: App Store incompatible, may break with macOS updates
- AppleScript automation - Rejected: Too slow for real-time window management
- Carbon Window Manager APIs - Rejected: Deprecated since macOS 10.15

**Implementation approach**:
- Primary: AXUIElement APIs for window positioning, resizing, and focus management
- Secondary: CGWindow APIs for window enumeration and property detection
- Fallback: CGSSpace private APIs only when standard APIs prove insufficient for Spaces integration

## Rust-macOS Integration

### Decision: Use cocoa and objc crates for Objective-C interop
**Rationale**: These crates provide mature, well-tested bindings to macOS frameworks with excellent memory safety guarantees when interfacing with Objective-C runtime.

**Alternatives considered**:
- Swift-Rust interop - Rejected: Adds unnecessary complexity, Swift not needed for system APIs
- Direct C FFI to Core Graphics - Rejected: More error-prone, less idiomatic Rust
- Bindgen for all APIs - Rejected: Manual bindings more maintainable for specific use case

**Key integration points**:
- cocoa crate for NSApplication, NSWindow, NSScreen APIs
- objc crate for direct Objective-C runtime interaction
- Core Graphics bindings for CGWindow, CGDisplay APIs

## Async Runtime and Event Handling

### Decision: Use tokio for async runtime with event-driven architecture
**Rationale**: Window management requires responsive handling of multiple concurrent events (hotkeys, window changes, monitor changes) while maintaining system responsiveness.

**Alternatives considered**:
- async-std - Rejected: tokio has better ecosystem support for system programming
- Blocking/synchronous architecture - Rejected: Would block UI during window operations
- Manual threading - Rejected: tokio provides better error handling and resource management

**Architecture**:
- Main tokio runtime for coordinating all operations
- Separate task for hotkey event monitoring
- Separate task for window change notifications
- Async message passing between components

## Configuration Storage

### Decision: File-based TOML configuration with JSON fallback
**Rationale**: Simple, human-readable configuration that doesn't require database setup. TOML provides excellent ergonomics for configuration files.

**Alternatives considered**:
- SQLite database - Rejected: Overkill for configuration data, adds complexity
- macOS Property Lists (plist) - Rejected: Less human-readable, harder to version control
- Registry/Preferences API - Rejected: Not cross-compatible, harder to backup/restore

**Schema**:
- Workspace definitions with keyboard shortcuts
- Tiling patterns and layout rules
- Monitor configuration and preferences
- Application-specific window rules

## Performance Monitoring

### Decision: Built-in metrics collection with optional external export
**Rationale**: Performance is critical for user experience. Built-in monitoring enables optimization and debugging without external dependencies.

**Alternatives considered**:
- No monitoring - Rejected: Performance requirements are strict
- External monitoring only - Rejected: Adds deployment complexity
- Logging-only approach - Rejected: Doesn't provide real-time performance insight

**Metrics to collect**:
- Workspace switch latency (target: <200ms)
- Window positioning time (target: <50ms)
- Memory usage tracking (limit: <100MB)
- Error rates and failure modes

## Error Handling Strategy

### Decision: Graceful degradation with user notification
**Rationale**: Window management failures should not crash the application. Users need clear feedback when operations fail due to permissions or system constraints.

**Approaches**:
- Permission failures: Clear notification with guidance to enable accessibility permissions
- API failures: Retry with exponential backoff, fallback to alternative APIs
- Application compatibility: Maintain allow/deny lists for problematic applications
- System state changes: Graceful handling of monitor changes, app crashes, etc.

## Testing Strategy

### Decision: Multi-layered testing with focus on integration scenarios
**Rationale**: Window management is inherently integration-heavy. Unit tests for business logic, integration tests for API interactions, and performance tests for timing requirements.

**Test layers**:
- Unit tests: Tiling algorithms, configuration parsing, keyboard mapping
- Integration tests: Window API interactions, workspace switching scenarios
- Performance tests: Latency measurements, memory usage validation
- Manual tests: Real-world usage scenarios, edge cases with specific applications

## Minimum macOS Version

### Decision: Target macOS 12 (Monterey) and later
**Rationale**: Provides access to modern Accessibility APIs while maintaining compatibility with reasonably recent systems. Most power users likely to use this tool will be on recent macOS versions.

**Alternatives considered**:
- macOS 10.15+ (Catalina) - Rejected: Missing some modern window management APIs
- macOS 13+ (Ventura) - Rejected: Unnecessarily excludes users on stable systems
- macOS 14+ (Sonoma) - Rejected: Too cutting-edge, reduces potential user base

**API availability verification**:
- Accessibility APIs: Available since macOS 10.4, stable since 10.9
- Core Graphics window APIs: Modern versions available since macOS 10.12
- Spaces APIs: Private APIs stable since macOS 10.5, public APIs limited

## Security and Permissions

### Decision: Request minimal required permissions with clear justification
**Rationale**: macOS security model requires explicit user consent for accessibility features. Clear communication about why permissions are needed builds user trust.

**Required permissions**:
- Accessibility: Essential for window manipulation and reading window properties
- Input Monitoring: Required for global hotkey handling
- Screen Recording: Only if advanced window content detection is needed

**Permission handling**:
- Check permissions on startup
- Provide clear instructions for enabling permissions
- Graceful operation with partial permissions where possible
- Regular permission status monitoring