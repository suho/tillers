
# Implementation Plan: TilleRS - Keyboard-First Tiling Window Manager

**Branch**: `001-build-an-application` | **Date**: 2025-09-21 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/Users/suho/Developer/suho/tillers/specs/001-build-an-application/spec.md`

## Execution Flow (/plan command scope)
```
1. Load feature spec from Input path
   → If not found: ERROR "No feature spec at {path}"
2. Fill Technical Context (scan for NEEDS CLARIFICATION)
   → Detect Project Type from context (web=frontend+backend, mobile=app+api)
   → Set Structure Decision based on project type
3. Fill the Constitution Check section based on the content of the constitution document.
4. Evaluate Constitution Check section below
   → If violations exist: Document in Complexity Tracking
   → If no justification possible: ERROR "Simplify approach first"
   → Update Progress Tracking: Initial Constitution Check
5. Execute Phase 0 → research.md
   → If NEEDS CLARIFICATION remain: ERROR "Resolve unknowns"
6. Execute Phase 1 → contracts, data-model.md, quickstart.md, agent-specific template file (e.g., `CLAUDE.md` for Claude Code, `.github/copilot-instructions.md` for GitHub Copilot, `GEMINI.md` for Gemini CLI, `QWEN.md` for Qwen Code or `AGENTS.md` for opencode).
7. Re-evaluate Constitution Check section
   → If new violations: Refactor design, return to Phase 1
   → Update Progress Tracking: Post-Design Constitution Check
8. Plan Phase 2 → Describe task generation approach (DO NOT create tasks.md)
9. STOP - Ready for /tasks command
```

**IMPORTANT**: The /plan command STOPS at step 7. Phases 2-4 are executed by other commands:
- Phase 2: /tasks command creates tasks.md
- Phase 3-4: Implementation execution (manual or via tools)

## Summary
TilleRS is a keyboard-first tiling window manager for macOS that automatically organizes windows into logical workspaces, enabling instant context switching between projects while maintaining predictable window layouts across multiple monitors. Built with Rust for performance and memory safety, utilizing macOS Accessibility APIs and Core Graphics through Objective-C interop to eliminate manual window management and keep users in flow state.

## Technical Context
**Language/Version**: Rust 1.75+  
**Primary Dependencies**: cocoa, objc (Objective-C interop), tokio (async runtime), Core Graphics framework, Accessibility APIs  
**Storage**: File-based configuration (JSON/TOML), no database required  
**Testing**: cargo test, integration tests for window management scenarios  
**Target Platform**: macOS 12+ (Monterey and later)
**Project Type**: single - native macOS application  
**Performance Goals**: <200ms workspace switching, <50ms window positioning response  
**Constraints**: <100MB memory usage, must respect macOS sandboxing and permissions, keyboard-only operation  
**Scale/Scope**: Single-user desktop application, 20+ configurable workspaces, unlimited windows per workspace

## Constitution Check
*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Code Quality Gates
- [x] All code changes will pass linting, formatting, and static analysis (clippy, rustfmt)
- [x] Technical debt will be documented and addressable within two cycles
- [x] Code quality metrics will be maintained or improved (Rust's safety guarantees)

### Testing Standards Compliance
- [x] TDD approach planned: tests written before implementation
- [x] Unit test coverage plan targets ≥90% for new code
- [x] Integration tests planned for all user workflows (workspace switching, window management)
- [x] Performance tests planned for critical paths (response time validation)

### User Experience Standards
- [x] Design patterns and accessibility standards identified (keyboard-first, predictable layouts)
- [x] Error message strategy defined (clear system notifications for failures)
- [x] Loading states and user feedback planned (visual indicators for workspace transitions)
- [x] WCAG 2.1 AA compliance approach defined (keyboard navigation, screen reader support)

### Performance Requirements
- [x] Response time targets ≤200ms for critical actions (workspace switching)
- [x] Memory usage limits defined and trackable (<100MB total)
- [x] Database query performance targets ≤50ms (file-based config, no DB queries)
- [x] Performance monitoring and alerting planned (metrics collection for response times)

### Observability Requirements
- [x] Logging strategy for critical paths defined (structured logging for window operations)
- [x] Error logging with debugging context planned (API failures, permission issues)
- [x] Performance metrics collection planned (timing metrics for all operations)
- [x] Health monitoring and dashboard approach defined (system health checks, error rates)

## Project Structure

### Documentation (this feature)
```
specs/[###-feature]/
├── plan.md              # This file (/plan command output)
├── research.md          # Phase 0 output (/plan command)
├── data-model.md        # Phase 1 output (/plan command)
├── quickstart.md        # Phase 1 output (/plan command)
├── contracts/           # Phase 1 output (/plan command)
└── tasks.md             # Phase 2 output (/tasks command - NOT created by /plan)
```

### Source Code (repository root)
```
# Option 1: Single project (DEFAULT)
src/
├── models/
├── services/
├── cli/
└── lib/

tests/
├── contract/
├── integration/
└── unit/

# Option 2: Web application (when "frontend" + "backend" detected)
backend/
├── src/
│   ├── models/
│   ├── services/
│   └── api/
└── tests/

frontend/
├── src/
│   ├── components/
│   ├── pages/
│   └── services/
└── tests/

# Option 3: Mobile + API (when "iOS/Android" detected)
api/
└── [same as backend above]

ios/ or android/
└── [platform-specific structure]
```

**Structure Decision**: Option 1 (Single project) - Native macOS application with modular Rust architecture

## Phase 0: Outline & Research
1. **Extract unknowns from Technical Context** above:
   - For each NEEDS CLARIFICATION → research task
   - For each dependency → best practices task
   - For each integration → patterns task

2. **Generate and dispatch research agents**:
   ```
   For each unknown in Technical Context:
     Task: "Research {unknown} for {feature context}"
   For each technology choice:
     Task: "Find best practices for {tech} in {domain}"
   ```

3. **Consolidate findings** in `research.md` using format:
   - Decision: [what was chosen]
   - Rationale: [why chosen]
   - Alternatives considered: [what else evaluated]

**Output**: research.md with all NEEDS CLARIFICATION resolved

## Phase 1: Design & Contracts
*Prerequisites: research.md complete*

1. **Extract entities from feature spec** → `data-model.md`:
   - Entity name, fields, relationships
   - Validation rules from requirements
   - State transitions if applicable

2. **Generate API contracts** from functional requirements:
   - For each user action → endpoint
   - Use standard REST/GraphQL patterns
   - Output OpenAPI/GraphQL schema to `/contracts/`

3. **Generate contract tests** from contracts:
   - One test file per endpoint
   - Assert request/response schemas
   - Tests must fail (no implementation yet)

4. **Extract test scenarios** from user stories:
   - Each story → integration test scenario
   - Quickstart test = story validation steps

5. **Update agent file incrementally** (O(1) operation):
   - Run `.specify/scripts/bash/update-agent-context.sh claude` for your AI assistant
   - If exists: Add only NEW tech from current plan
   - Preserve manual additions between markers
   - Update recent changes (keep last 3)
   - Keep under 150 lines for token efficiency
   - Output to repository root

**Output**: data-model.md, /contracts/*, failing tests, quickstart.md, agent-specific file

## Phase 2: Task Planning Approach
*This section describes what the /tasks command will do - DO NOT execute during /plan*

**Task Generation Strategy**:
- Load `.specify/templates/tasks-template.md` as base
- Generate tasks from Phase 1 design docs (contracts, data model, quickstart)
- From contracts: 3 API contract test tasks (workspace_manager, window_manager, keyboard_handler) [P]
- From data model: 6 entity model creation tasks (Workspace, TilingPattern, WindowRule, MonitorConfiguration, KeyboardMapping, ApplicationProfile) [P]
- From quickstart: 6 integration test scenarios from user acceptance tests
- Implementation tasks to make all tests pass (estimated 12-15 tasks)

**Ordering Strategy**:
- TDD order: All contract tests → All integration tests → Model implementations → Service implementations → CLI implementation
- Dependency order: Data models → Core services → Window management → Keyboard handling → Configuration UI
- Mark [P] for parallel execution: All tests, all models, independent service modules
- Sequential: Service integration, final system assembly

**Rust-Specific Task Considerations**:
- Cargo project setup with proper workspace structure
- Objective-C interop setup for macOS APIs
- Permission handling and entitlements configuration
- Performance benchmarking tasks for SLA validation
- macOS app bundle creation and signing

**Estimated Output**: 28-32 numbered, ordered tasks in tasks.md

**Key Parallel Execution Groups**:
1. Contract tests (3 tasks) - can run simultaneously
2. Data model implementation (6 tasks) - independent modules
3. Core service implementation (4 tasks) - after models complete
4. Integration tests (6 tasks) - can run after basic implementation

**IMPORTANT**: This phase is executed by the /tasks command, NOT by /plan

## Phase 3+: Future Implementation
*These phases are beyond the scope of the /plan command*

**Phase 3**: Task execution (/tasks command creates tasks.md)  
**Phase 4**: Implementation (execute tasks.md following constitutional principles)  
**Phase 5**: Validation (run tests, execute quickstart.md, performance validation)

## Complexity Tracking
*Fill ONLY if Constitution Check has violations that must be justified*

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| [e.g., 4th project] | [current need] | [why 3 projects insufficient] |
| [e.g., Repository pattern] | [specific problem] | [why direct DB access insufficient] |


## Progress Tracking
*This checklist is updated during execution flow*

**Phase Status**:
- [x] Phase 0: Research complete (/plan command)
- [x] Phase 1: Design complete (/plan command)
- [x] Phase 2: Task planning complete (/plan command - describe approach only)
- [ ] Phase 3: Tasks generated (/tasks command)
- [ ] Phase 4: Implementation complete
- [ ] Phase 5: Validation passed

**Gate Status**:
- [x] Initial Constitution Check: PASS
- [x] Post-Design Constitution Check: PASS
- [x] All NEEDS CLARIFICATION resolved
- [x] Complexity deviations documented

---
*Based on Constitution v1.0.0 - See `.specify/memory/constitution.md`*
