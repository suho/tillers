# TilleRS Quickstart Guide & Test Scenarios
**Updated**: 2025-09-21 - Changed default keyboard shortcuts from Command to Option key

## Installation and Setup

### Prerequisites
- macOS 12 (Monterey) or later
- Accessibility permissions enabled
- Input monitoring permissions enabled

### Initial Setup Process
1. Launch TilleRS application
2. Grant accessibility permissions when prompted
3. Grant input monitoring permissions for hotkey functionality
4. Complete initial configuration wizard

### First-Time Configuration
1. **Create Default Workspace**
   - Open workspace configuration
   - Create workspace named "Development" with shortcut `Option+1` (changed from Cmd+1)
   - Assign "Two Column" tiling pattern
   - Save configuration

2. **Test Basic Functionality**
   - Open 2-3 applications (e.g., Terminal, Browser, Text Editor)
   - Switch to "Development" workspace using `Option+1` (changed from Cmd+1)
   - Verify windows arrange automatically in two-column layout

## User Acceptance Test Scenarios

### Scenario 1: Basic Workspace Creation and Switching
**Objective**: Verify user can create workspaces and switch between them

**Steps**:
1. Launch TilleRS
2. Create workspace "Project A" with shortcut `Option+1` (changed from Cmd+1)
3. Create workspace "Project B" with shortcut `Option+2` (changed from Cmd+2) 
4. Open VS Code, Terminal, and Safari
5. Switch to "Project A" using `Option+1`
6. Verify all windows arrange according to default tiling pattern
7. Switch to "Project B" using `Option+2`
8. Verify workspace transition is smooth and windows maintain positions

**Expected Results**:
- Workspace switching completes within 200ms
- Windows automatically arrange in predefined layouts
- No manual window positioning required
- Workspace state persists between switches

### Scenario 2: Multi-Monitor Workspace Management
**Objective**: Verify workspace behavior across multiple monitors

**Prerequisites**: Two monitors connected

**Steps**:
1. Create workspace "Multi-Monitor" with shortcut `Option+3` (changed from Cmd+3)
2. Configure different tiling patterns for each monitor
3. Open 6 applications total
4. Switch to "Multi-Monitor" workspace
5. Verify windows distribute across both monitors according to configuration
6. Disconnect secondary monitor
7. Verify graceful handling of monitor change
8. Reconnect monitor
9. Verify workspace layout restoration

**Expected Results**:
- Windows distribute correctly across monitors
- Monitor changes handled gracefully without crashes
- Layout restoration when monitor reconnected
- No windows lost or hidden during monitor changes

### Scenario 3: Application-Specific Window Rules
**Objective**: Verify custom rules for specific applications work correctly

**Steps**:
1. Create workspace "Design Work" with shortcut `Option+4` (changed from Cmd+4)
2. Configure window rule: Figma → always fullscreen
3. Configure window rule: Finder → always floating
4. Configure window rule: Slack → fixed position in corner
5. Open Figma, Finder, and Slack
6. Switch to "Design Work" workspace
7. Verify each application follows its configured rule

**Expected Results**:
- Figma opens in fullscreen mode
- Finder remains floating and user-positionable
- Slack positions in designated corner
- Other applications follow default tiling pattern

### Scenario 4: Keyboard-Only Navigation
**Objective**: Verify complete keyboard-only operation

**Steps**:
1. Remove mouse/trackpad access (disable or move away)
2. Launch TilleRS using keyboard launcher
3. Create new workspace using keyboard shortcuts
4. Configure tiling pattern using keyboard navigation
5. Open applications using Spotlight/launcher
6. Switch between workspaces using shortcuts
7. Navigate between windows within workspace using shortcuts
8. Modify window positions using keyboard commands

**Expected Results**:
- All operations completable without mouse
- Keyboard shortcuts respond immediately
- Clear feedback for all keyboard actions
- No functionality requires mouse interaction

### Scenario 5: Performance Under Load
**Objective**: Verify performance with many windows and workspaces

**Steps**:
1. Create 10 different workspaces with unique shortcuts
2. Open 20+ application windows across different workspaces
3. Rapidly switch between workspaces (5 switches per second for 30 seconds)
4. Monitor memory usage and response times
5. Add new windows while switching workspaces
6. Test workspace switching with high CPU load (video rendering, compilation)

**Expected Results**:
- Workspace switching remains under 200ms throughout test
- Memory usage stays under 100MB
- No crashes or freezes during rapid switching
- Performance degrades gracefully under high system load

### Scenario 6: Error Handling and Recovery
**Objective**: Verify graceful handling of error conditions

**Steps**:
1. Create workspace with non-existent tiling pattern
2. Attempt to use keyboard shortcut already assigned to system
3. Try to create workspace while accessibility permissions disabled
4. Force-quit application that has windows in active workspace
5. Attempt workspace switch while another app is in fullscreen mode
6. Try to tile application that doesn't support window management

**Expected Results**:
- Clear error messages for each failure condition
- Application continues running after errors
- Graceful fallback behaviors when operations fail
- User guidance provided for permission issues

## Integration Test Scenarios

### Window Management Integration
**Test**: Window positioning accuracy
1. Create test workspace with precise window positions
2. Measure actual vs. expected window coordinates
3. Verify sub-pixel positioning accuracy
4. Test with different monitor DPI settings

**Test**: Application compatibility
1. Test with 20+ common macOS applications
2. Verify window detection and manipulation works correctly
3. Document any incompatible applications
4. Test with both native and Electron-based apps

### System Integration
**Test**: macOS permission handling
1. Test permission request flow
2. Verify graceful degradation without permissions
3. Test permission re-request after initial denial
4. Verify sandbox compatibility if applicable

**Test**: Performance monitoring
1. Verify all metrics collection functions work
2. Test performance under various system loads
3. Validate memory leak detection
4. Test error rate tracking accuracy

## Manual Testing Checklist

### Basic Functionality
- [ ] Application launches successfully on clean system
- [ ] Initial setup wizard completes without errors
- [ ] Default configuration created correctly
- [ ] All keyboard shortcuts register successfully
- [ ] Window detection works for all tested applications
- [ ] Workspace switching time consistently under 200ms
- [ ] Memory usage remains under 100MB during normal operation

### User Experience
- [ ] All error messages are clear and actionable
- [ ] Visual feedback provided for all user actions
- [ ] Keyboard navigation is intuitive and complete
- [ ] Configuration UI is accessible via keyboard
- [ ] Help documentation is accessible and complete

### Edge Cases
- [ ] Handles monitor configuration changes gracefully
- [ ] Recovers correctly from application crashes
- [ ] Manages focus correctly with fullscreen applications
- [ ] Handles rapid user input without dropping commands
- [ ] Maintains configuration integrity across app restarts

### Performance Validation
- [ ] Startup time under 3 seconds
- [ ] Configuration load time under 1 second
- [ ] Workspace switch time under 200ms (95th percentile)
- [ ] Window positioning time under 50ms
- [ ] Memory usage growth rate acceptable over extended use

## Automated Test Execution

### Unit Test Coverage
- Configuration parsing and validation: ≥95%
- Tiling algorithms: ≥90%
- Keyboard shortcut processing: ≥95%
- Error handling: ≥85%

### Integration Test Requirements
- Window management API interactions: All critical paths covered
- Permission handling flows: All scenarios tested
- Multi-monitor support: Primary use cases validated
- Performance benchmarks: All SLA targets verified

### Performance Test Automation
- Memory usage monitoring during extended operation
- Response time measurement for all user actions
- Stress testing with maximum window counts
- Load testing under high system resource usage