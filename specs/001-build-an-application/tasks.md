# Tasks: TilleRS - Keyboard-First Tiling Window Manager

**Input**: Design documents from `/Users/suho/Developer/suho/tillers/specs/001-build-an-application/`
**Prerequisites**: plan.md (required), research.md, data-model.md, contracts/

## Execution Flow (main)
```
1. Load plan.md from feature directory
   → If not found: ERROR "No implementation plan found"
   → Extract: tech stack, libraries, structure
2. Load optional design documents:
   → data-model.md: Extract entities → model tasks
   → contracts/: Each file → contract test task
   → research.md: Extract decisions → setup tasks
3. Generate tasks by category:
   → Setup: project init, dependencies, linting
   → Tests: contract tests, integration tests
   → Core: models, services, CLI commands
   → Integration: DB, middleware, logging
   → Polish: unit tests, performance, docs
4. Apply task rules:
   → Different files = mark [P] for parallel
   → Same file = sequential (no [P])
   → Tests before implementation (TDD)
5. Number tasks sequentially (T001, T002...)
6. Generate dependency graph
7. Create parallel execution examples
8. Validate task completeness:
   → All contracts have tests?
   → All entities have models?
   → All endpoints implemented?
9. Return: SUCCESS (tasks ready for execution)
```

## Format: `[ID] [P?] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- Include exact file paths in descriptions

## Path Conventions
- **Single project**: `src/`, `tests/` at repository root
- Rust project structure with Cargo workspace
- macOS app bundle structure in target/

## Phase 3.1: Setup
- [x] T001 Create Rust project structure with Cargo.toml and workspace configuration
- [x] T002 Initialize Rust project with macOS dependencies (cocoa, objc, tokio, serde, uuid)
- [x] T003 [P] Configure Rust linting (clippy, rustfmt) and CI pipeline
- [x] T004 [P] Setup macOS app bundle structure and Info.plist configuration
- [x] T005 [P] Configure macOS entitlements for accessibility and input monitoring permissions

## Phase 3.2: Tests First (TDD) ⚠️ MUST COMPLETE BEFORE 3.3
**CRITICAL: These tests MUST be written and MUST FAIL before ANY implementation**

### Contract Tests
- [x] T006 [P] Contract test workspace manager API in tests/contract/test_workspace_manager.rs
- [x] T007 [P] Contract test window manager API in tests/contract/test_window_manager.rs  
- [x] T008 [P] Contract test keyboard handler API (Option modifier default + conflict detection) in tests/contract/test_keyboard_handler.rs

### Integration Tests (from quickstart scenarios)
- [x] T009 [P] Integration test basic workspace creation and switching in tests/integration/test_workspace_switching.rs
- [x] T010 [P] Integration test multi-monitor workspace management in tests/integration/test_multi_monitor.rs
- [x] T011 [P] Integration test application-specific window rules in tests/integration/test_window_rules.rs
- [x] T012 [P] Integration test keyboard-only navigation in tests/integration/test_keyboard_navigation.rs
- [x] T013 [P] Integration test performance under load in tests/integration/test_performance.rs
- [x] T014 [P] Integration test error handling and recovery in tests/integration/test_error_handling.rs

## Phase 3.3: Core Implementation (ONLY after tests are failing)

### Data Models
- [x] T015 [P] Workspace model with validation and serialization in src/models/workspace.rs
- [x] T016 [P] TilingPattern model with layout algorithms in src/models/tiling_pattern.rs
- [x] T017 [P] WindowRule model for application-specific behavior in src/models/window_rule.rs
- [x] T018 [P] MonitorConfiguration model for multi-display support in src/models/monitor_configuration.rs
- [x] T019 [P] KeyboardMapping model enforcing Option (⌥) default modifiers and uniqueness in src/models/keyboard_mapping.rs
- [x] T020 [P] ApplicationProfile model for app compatibility in src/models/application_profile.rs

### Core Services
- [x] T021 WorkspaceManager service for workspace CRUD and switching in src/services/workspace_manager.rs
- [x] T022 WindowManager service for macOS window manipulation via Accessibility APIs in src/services/window_manager.rs
- [x] T023 KeyboardHandler service for global hotkey registration (Option-first combos, legacy Command migration) in src/services/keyboard_handler.rs
- [x] T024 TilingEngine service for window layout calculation and positioning in src/services/tiling_engine.rs

### macOS Integration Layer
- [x] T025 [P] Accessibility API wrapper for window detection and manipulation in src/macos/accessibility.rs
- [x] T026 [P] Core Graphics API wrapper for monitor enumeration and geometry in src/macos/core_graphics.rs
- [x] T027 [P] Objective-C bridge layer for NSApplication and system integration in src/macos/objc_bridge.rs

### Configuration Management
- [x] T028 Configuration parser for TOML workspace definitions (validate Option default, warn on Command legacy) in src/config/parser.rs
- [x] T029 Configuration validator with schema validation in src/config/validator.rs
- [x] T030 Configuration persistence with atomic file operations in src/config/persistence.rs

## Phase 3.4: Integration
- [x] T031 Connect WorkspaceManager to file-based configuration storage with migration for Command→Option shortcuts
- [x] T032 Integrate TilingEngine with WindowManager for automatic layout
- [x] T033 Connect KeyboardHandler to WorkspaceManager for shortcut-triggered actions (ensure Option defaults applied and conflicts resolved)
- [x] T034 Add structured logging with configurable levels using tracing crate
- [ ] T035 Implement permission checker for accessibility and input monitoring
- [ ] T036 Add error recovery for API failures and permission issues

## Phase 3.5: Application Assembly
- [ ] T037 Main application entry point with async runtime setup in src/main.rs
- [ ] T038 System tray integration for status and basic controls in src/ui/system_tray.rs
- [ ] T039 CLI interface for configuration and debugging in src/cli/mod.rs
- [ ] T040 Application lifecycle management (startup, shutdown, background mode)

## Phase 3.6: Polish
- [ ] T041 [P] Unit tests for tiling algorithms in tests/unit/test_tiling_algorithms.rs
- [ ] T042 [P] Unit tests for configuration parsing in tests/unit/test_config_parser.rs
- [ ] T043 [P] Performance benchmarks for workspace switching (<200ms) in benches/workspace_switching.rs
- [ ] T044 [P] Performance benchmarks for window positioning (<50ms) in benches/window_positioning.rs
- [ ] T045 [P] Memory usage monitoring and leak detection in tests/performance/test_memory.rs
- [ ] T046 Documentation generation with rustdoc for public APIs
- [ ] T047 Create installation guide and user documentation detailing Option-key shortcut defaults and migration guidance
- [ ] T048 Run complete quickstart manual testing scenarios

## Dependencies
- Setup (T001-T005) before everything
- Contract tests (T006-T008) before any implementation
- Integration tests (T009-T014) before core implementation
- Models (T015-T020) before services (T021-T024)
- macOS integration (T025-T027) supports services
- Configuration (T028-T030) before workspace persistence (T031)
- Core services before integration (T031-T036)
- Integration before application assembly (T037-T040)
- Implementation before polish (T041-T048)

## Parallel Example
```
# Launch contract tests together:
Task: "Contract test workspace manager API in tests/contract/test_workspace_manager.rs"
Task: "Contract test window manager API in tests/contract/test_window_manager.rs"  
Task: "Contract test keyboard handler API in tests/contract/test_keyboard_handler.rs"

# Launch model creation tasks together:
Task: "Workspace model with validation and serialization in src/models/workspace.rs"
Task: "TilingPattern model with layout algorithms in src/models/tiling_pattern.rs"
Task: "WindowRule model for application-specific behavior in src/models/window_rule.rs"
Task: "MonitorConfiguration model for multi-display support in src/models/monitor_configuration.rs"
Task: "KeyboardMapping model for shortcut management in src/models/keyboard_mapping.rs"
Task: "ApplicationProfile model for app compatibility in src/models/application_profile.rs"

# Launch macOS integration tasks together:
Task: "Accessibility API wrapper for window detection and manipulation in src/macos/accessibility.rs"
Task: "Core Graphics API wrapper for monitor enumeration and geometry in src/macos/core_graphics.rs"
Task: "Objective-C bridge layer for NSApplication and system integration in src/macos/objc_bridge.rs"
```

## Notes
- [P] tasks = different files, no dependencies
- Verify tests fail before implementing
- Use `cargo test` to run all tests
- Use `cargo clippy` for linting
- Use `cargo fmt` for formatting
- Commit after each task completion
- Focus on TDD: Red → Green → Refactor

## Task Generation Rules
*Applied during main() execution*

1. **From Contracts**:
   - workspace_manager.yaml → contract test task [P]
   - window_manager.yaml → contract test task [P]
   - keyboard_handler.yaml → contract test task [P]
   
2. **From Data Model**:
   - Workspace → model creation task [P]
   - TilingPattern → model creation task [P]
   - WindowRule → model creation task [P]
   - MonitorConfiguration → model creation task [P]
   - KeyboardMapping → model creation task [P]
   - ApplicationProfile → model creation task [P]
   
3. **From User Stories (quickstart.md)**:
   - Basic workspace creation → integration test [P]
   - Multi-monitor management → integration test [P]
   - Application-specific rules → integration test [P]
   - Keyboard-only navigation → integration test [P]
   - Performance under load → integration test [P]
   - Error handling → integration test [P]

4. **Ordering**:
   - Setup → Tests → Models → Services → Integration → Assembly → Polish
   - macOS dependencies require sequential setup due to permission requirements

## Validation Checklist
*GATE: Checked by main() before returning*

- [x] All contracts have corresponding tests (T006-T008)
- [x] All entities have model tasks (T015-T020)
- [x] All tests come before implementation (T006-T014 before T015+)
- [x] Parallel tasks truly independent (different files, no shared state)
- [x] Each task specifies exact file path
- [x] No task modifies same file as another [P] task
- [x] Performance requirements covered (T043-T045)
- [x] macOS-specific considerations included (T025-T027, T035)
- [x] TDD workflow enforced (tests must fail before implementation)
