// Contract: Window Management API
// Updated: 2025-09-21 - Added Option key support for window operations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Move focus to a different window
#[derive(Debug, Serialize, Deserialize)]
pub struct MoveFocusRequest {
    pub direction: FocusDirection,
    pub wrap_around: bool, // If true, wraps to opposite edge when at boundary
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MoveFocusResponse {
    pub previous_window_id: Option<String>,
    pub new_window_id: Option<String>,
    pub focus_change_duration_ms: u64,
}

/// Move a window within the tiling layout
#[derive(Debug, Serialize, Deserialize)]
pub struct MoveWindowRequest {
    pub window_id: String,
    pub direction: MoveDirection,
    pub swap_with_target: bool, // If true, swaps positions; if false, pushes other windows
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MoveWindowResponse {
    pub new_position: TilePosition,
    pub affected_windows: Vec<WindowPositionChange>,
    pub move_duration_ms: u64,
}

/// Resize a window within its tile
#[derive(Debug, Serialize, Deserialize)]
pub struct ResizeWindowRequest {
    pub window_id: String,
    pub resize_type: ResizeType,
    pub amount: ResizeAmount,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResizeWindowResponse {
    pub new_size: WindowSize,
    pub affected_windows: Vec<WindowPositionChange>,
    pub resize_duration_ms: u64,
}

/// Toggle a window between tiled and floating modes
#[derive(Debug, Serialize, Deserialize)]
pub struct ToggleFloatRequest {
    pub window_id: String,
    pub restore_to_tile: bool, // When floating, whether to remember tile position
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToggleFloatResponse {
    pub new_mode: WindowMode,
    pub previous_position: Option<TilePosition>,
    pub new_position: WindowPosition,
}

/// Get information about a specific window
#[derive(Debug, Serialize, Deserialize)]
pub struct GetWindowInfoRequest {
    pub window_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetWindowInfoResponse {
    pub window: WindowInfo,
    pub current_workspace: String,
    pub tiling_status: TilingStatus,
}

/// List all windows in current workspace
#[derive(Debug, Serialize, Deserialize)]
pub struct ListWindowsRequest {
    pub workspace_id: Option<String>, // If None, uses current workspace
    pub include_hidden: bool,
    pub sort_by: Option<WindowSortField>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListWindowsResponse {
    pub windows: Vec<WindowSummary>,
    pub workspace_id: String,
    pub total_count: u32,
}

/// Apply window rules to current or specific windows
#[derive(Debug, Serialize, Deserialize)]
pub struct ApplyWindowRulesRequest {
    pub window_ids: Option<Vec<String>>, // If None, applies to all windows
    pub force_reapply: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApplyWindowRulesResponse {
    pub rules_applied: u32,
    pub windows_affected: Vec<String>,
    pub errors: Vec<WindowRuleError>,
}

// Supporting types

#[derive(Debug, Serialize, Deserialize)]
pub enum FocusDirection {
    Left,
    Right,
    Up,
    Down,
    Next,     // Cycle to next window in order
    Previous, // Cycle to previous window in order
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MoveDirection {
    Left,
    Right,
    Up,
    Down,
    ToPosition(TilePosition), // Move to specific tile position
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ResizeType {
    Grow,   // Make window larger
    Shrink, // Make window smaller
    Set,    // Set to specific size
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ResizeAmount {
    Small,           // 10% change
    Medium,          // 25% change
    Large,           // 50% change
    Pixels(i32),     // Exact pixel amount
    Percentage(f32), // Percentage change
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WindowMode {
    Tiled,
    Floating,
    Fullscreen,
    Minimized,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TilePosition {
    pub column: u32,
    pub row: u32,
    pub span_columns: u32,
    pub span_rows: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: String,
    pub title: String,
    pub application_name: String,
    pub bundle_id: String,
    pub position: WindowPosition,
    pub mode: WindowMode,
    pub is_focused: bool,
    pub is_visible: bool,
    pub workspace_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowSummary {
    pub id: String,
    pub title: String,
    pub application_name: String,
    pub mode: WindowMode,
    pub is_focused: bool,
    pub tile_position: Option<TilePosition>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowPositionChange {
    pub window_id: String,
    pub old_position: WindowPosition,
    pub new_position: WindowPosition,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TilingStatus {
    Tiled { position: TilePosition },
    Floating,
    Excluded, // Window is ignored by tiling system
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WindowSortField {
    Title,
    ApplicationName,
    LastFocused,
    Position,
    Size,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WindowRuleError {
    ApplicationNotFound { bundle_id: String },
    WindowNotFound { window_id: String },
    InvalidRule { rule_id: String, reason: String },
    PermissionDenied { required_permission: String },
    TilingFailed { window_id: String, details: String },
}

// Error types for window operations
#[derive(Debug, Serialize, Deserialize)]
pub enum WindowError {
    NotFound { window_id: String },
    NotAccessible { window_id: String, reason: String },
    InvalidOperation { operation: String, reason: String },
    TilingNotAvailable { workspace_id: String },
    ResizeFailed { window_id: String, details: String },
    MoveFailed { window_id: String, details: String },
    PermissionDenied { required_permission: String },
}

// Contract tests (these should fail initially)
#[cfg(test)]
mod contract_tests {
    use super::*;

    #[test]
    fn test_move_focus_with_option_key_shortcuts() {
        let request = MoveFocusRequest {
            direction: FocusDirection::Right,
            wrap_around: true,
        };
        
        // This test should fail until focus movement is implemented
        // Should support Option+Arrow key shortcuts for focus movement
        assert!(false, "Focus movement not yet implemented");
    }

    #[test]
    fn test_window_move_performance() {
        let request = MoveWindowRequest {
            window_id: "test_window".to_string(),
            direction: MoveDirection::Left,
            swap_with_target: true,
        };
        
        // This test should fail until window movement is implemented
        // Performance requirement: <50ms for window positioning
        assert!(false, "Window movement not yet implemented");
    }

    #[test]
    fn test_resize_window_proportional() {
        let request = ResizeWindowRequest {
            window_id: "test_window".to_string(),
            resize_type: ResizeType::Grow,
            amount: ResizeAmount::Medium,
        };
        
        // This test should fail until window resizing is implemented
        assert!(false, "Window resizing not yet implemented");
    }

    #[test]
    fn test_toggle_float_state_preservation() {
        let request = ToggleFloatRequest {
            window_id: "test_window".to_string(),
            restore_to_tile: true,
        };
        
        // This test should fail until float toggling is implemented
        assert!(false, "Float toggle not yet implemented");
    }

    #[test]
    fn test_accessibility_permission_requirement() {
        // Should fail gracefully when accessibility permissions are not granted
        let request = GetWindowInfoRequest {
            window_id: "test_window".to_string(),
        };
        
        // This test should fail until permission handling is implemented
        assert!(false, "Permission handling not yet implemented");
    }
}