# Data Model: TilleRS Window Manager

## Core Entities

### Workspace
**Purpose**: Represents a logical grouping of applications and windows with defined layout rules

**Fields**:
- `id`: Unique identifier (UUID)
- `name`: Human-readable workspace name
- `description`: Optional workspace description
- `keyboard_shortcut`: Key combination for switching to this workspace
- `tiling_pattern_id`: Reference to default tiling pattern
- `monitor_assignments`: Map of monitor IDs to layout preferences
- `auto_arrange`: Boolean flag for automatic window arrangement
- `created_at`: Timestamp of workspace creation
- `last_used`: Timestamp of last activation

**Relationships**:
- Has many WindowRules (defines behavior for specific applications)
- References one TilingPattern as default
- Has many MonitorConfigurations

**Validation Rules**:
- Name must be unique per user
- Keyboard shortcut must be unique system-wide
- Monitor assignments must reference valid monitors

**State Transitions**:
- Inactive → Active (when user switches to workspace)
- Active → Inactive (when user switches away)
- Active → Modified (when window layout changes)

### TilingPattern
**Purpose**: Defines how windows should be arranged within a workspace

**Fields**:
- `id`: Unique identifier (UUID)
- `name`: Pattern name (e.g., "Two Column", "Master-Stack", "Grid")
- `layout_algorithm`: Algorithm type (enum: MasterStack, Grid, Columns, Custom)
- `main_area_ratio`: Percentage of screen for main window area (0.0-1.0)
- `gap_size`: Pixel gap between windows
- `window_margin`: Pixel margin around windows
- `max_windows`: Maximum windows before pattern adjustment
- `resize_behavior`: How pattern adapts to new windows (enum: Shrink, Stack, Overflow)

**Relationships**:
- Used by many Workspaces
- Has many PatternRules for customization

**Validation Rules**:
- Main area ratio must be between 0.1 and 0.9
- Gap size and margin must be non-negative
- Max windows must be positive integer

### WindowRule
**Purpose**: Defines how specific applications or window types behave within a workspace

**Fields**:
- `id`: Unique identifier (UUID)
- `workspace_id`: Reference to parent workspace
- `application_identifier`: Bundle ID or process name pattern
- `window_title_pattern`: Regex pattern for window titles (optional)
- `positioning_rule`: How window should be positioned (enum: Auto, Fixed, Floating, Fullscreen)
- `fixed_position`: Specific coordinates if positioning_rule is Fixed
- `fixed_size`: Specific dimensions if positioning_rule is Fixed
- `z_order_priority`: Layer priority for window stacking
- `focus_behavior`: Auto-focus rules (enum: Never, OnCreate, OnSwitch)

**Relationships**:
- Belongs to one Workspace
- May reference ApplicationProfile for defaults

**Validation Rules**:
- Application identifier must be valid bundle ID or process name
- Window title pattern must be valid regex
- Fixed position and size must be within screen bounds
- Z-order priority must be non-negative

### MonitorConfiguration
**Purpose**: Defines workspace behavior for specific monitor setups

**Fields**:
- `id`: Unique identifier (UUID)
- `workspace_id`: Reference to parent workspace
- `monitor_identifier`: System monitor ID or display name
- `primary_pattern_id`: Main tiling pattern for this monitor
- `secondary_pattern_id`: Fallback pattern for overflow windows
- `active_area`: Screen area to use for tiling (excludes dock, menu bar)
- `orientation_preference`: Preferred monitor orientation
- `scale_factor`: DPI scaling factor for window sizing

**Relationships**:
- Belongs to one Workspace
- References TilingPatterns for layout

**Validation Rules**:
- Monitor identifier must correspond to available display
- Active area must be within monitor bounds
- Scale factor must be positive

### KeyboardMapping
**Purpose**: Defines keyboard shortcuts for workspace and window operations

**Fields**:
- `id`: Unique identifier (UUID)
- `shortcut_combination`: Key combination (modifiers + key)
- `action_type`: Type of action (enum: SwitchWorkspace, MoveWindow, ResizeWindow, CreateWorkspace)
- `target_id`: ID of target workspace/window (if applicable)
- `parameters`: JSON blob for action-specific parameters
- `enabled`: Boolean flag for shortcut activation
- `global_scope`: Whether shortcut works globally or only when app is focused

**Relationships**:
- May reference Workspace for switch actions
- May reference WindowRule for window-specific actions

**Validation Rules**:
- Shortcut combination must be unique within scope
- Action type must match target_id type
- Parameters must be valid JSON

### ApplicationProfile
**Purpose**: Stores default behavior patterns for specific applications

**Fields**:
- `id`: Unique identifier (UUID)
- `bundle_identifier`: macOS bundle ID
- `display_name`: Human-readable application name
- `default_positioning`: Default positioning rule for new windows
- `preferred_tiling_patterns`: List of compatible tiling patterns
- `compatibility_notes`: Known issues or special handling requirements
- `window_detection_rules`: Rules for identifying application windows
- `focus_stealing_behavior`: How app handles focus changes

**Relationships**:
- Used by WindowRules for defaults
- Has many CompatibilityRules

**Validation Rules**:
- Bundle identifier must be unique
- Preferred patterns must reference valid TilingPattern IDs

## Data Relationships

```
Workspace (1) -----> (many) WindowRule
Workspace (1) -----> (many) MonitorConfiguration
Workspace (many) --> (1) TilingPattern [default]

TilingPattern (1) --> (many) Workspace
TilingPattern (1) --> (many) PatternRule

WindowRule (many) --> (1) ApplicationProfile [optional]
KeyboardMapping (many) --> (1) Workspace [for switch actions]

MonitorConfiguration (many) --> (1) TilingPattern [primary]
MonitorConfiguration (many) --> (1) TilingPattern [secondary]
```

## Persistence Format

**Storage**: TOML configuration files with JSON schema validation

**File Structure**:
```
~/.config/tillers/
├── workspaces.toml          # Workspace definitions
├── patterns.toml            # Tiling pattern library
├── applications.toml        # Application profiles
├── keybindings.toml        # Keyboard shortcuts
└── monitors.toml           # Monitor configurations
```

**Backup Strategy**:
- Automatic backup on configuration changes
- Versioned configuration files
- Export/import functionality for sharing configurations

## Runtime State

**In-Memory State**:
- Current active workspace
- Window position cache
- Monitor change detection
- Application focus history
- Performance metrics buffer

**State Persistence**:
- Last active workspace saved on exit
- Window positions cached for quick restoration
- Performance metrics persisted for analysis