// Contract: Workspace Management API
// Updated: 2025-09-21 - Changed default hotkey modifier from Command to Option

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Create a new workspace
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub description: Option<String>,
    /// Keyboard shortcut with Option key as default modifier (instead of Command)
    pub keyboard_shortcut: KeyboardShortcut,
    pub tiling_pattern_id: Option<String>,
    pub auto_arrange: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateWorkspaceResponse {
    pub workspace_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Switch to an existing workspace
#[derive(Debug, Serialize, Deserialize)]
pub struct SwitchWorkspaceRequest {
    pub workspace_id: String,
    pub preserve_window_state: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwitchWorkspaceResponse {
    pub previous_workspace_id: Option<String>,
    pub switch_duration_ms: u64,
    pub windows_arranged: u32,
}

/// Update workspace configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateWorkspaceRequest {
    pub workspace_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    /// Updated keyboard shortcut (using Option key by default)
    pub keyboard_shortcut: Option<KeyboardShortcut>,
    pub tiling_pattern_id: Option<String>,
    pub auto_arrange: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateWorkspaceResponse {
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub changes_applied: Vec<String>,
}

/// List all workspaces
#[derive(Debug, Serialize, Deserialize)]
pub struct ListWorkspacesRequest {
    pub include_inactive: bool,
    pub sort_by: Option<WorkspaceSortField>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListWorkspacesResponse {
    pub workspaces: Vec<WorkspaceSummary>,
    pub active_workspace_id: Option<String>,
}

/// Delete a workspace
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteWorkspaceRequest {
    pub workspace_id: String,
    pub move_windows_to: Option<String>, // Target workspace ID for orphaned windows
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteWorkspaceResponse {
    pub deleted_at: chrono::DateTime<chrono::Utc>,
    pub windows_moved: u32,
}

// Supporting types

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyboardShortcut {
    /// Primary modifier keys - Option key is now the default instead of Command
    pub modifiers: Vec<ModifierKey>,
    /// Main key to press
    pub key: String,
    /// Whether this is a global shortcut (works when app not focused)
    pub global: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ModifierKey {
    /// Option key (⌥) - NEW DEFAULT MODIFIER
    Option,
    /// Command key (⌘) - Available for compatibility
    Command,
    /// Control key (⌃)
    Control,
    /// Shift key (⇧)
    Shift,
    /// Function key (fn)
    Function,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub window_count: u32,
    pub last_used: chrono::DateTime<chrono::Utc>,
    pub is_active: bool,
    /// Keyboard shortcut using updated modifier scheme
    pub keyboard_shortcut: KeyboardShortcut,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WorkspaceSortField {
    Name,
    LastUsed,
    WindowCount,
    CreatedAt,
}

// Error types for workspace operations
#[derive(Debug, Serialize, Deserialize)]
pub enum WorkspaceError {
    NotFound { workspace_id: String },
    InvalidName { reason: String },
    DuplicateShortcut { conflicting_workspace: String },
    SystemShortcutConflict { system_shortcut: String },
    TilingPatternNotFound { pattern_id: String },
    PermissionDenied { required_permission: String },
    WindowManagementFailed { details: String },
}

// Contract tests (these should fail initially)
#[cfg(test)]
mod contract_tests {
    use super::*;

    #[test]
    fn test_create_workspace_with_option_key() {
        let request = CreateWorkspaceRequest {
            name: "Development".to_string(),
            description: Some("Main development workspace".to_string()),
            keyboard_shortcut: KeyboardShortcut {
                modifiers: vec![ModifierKey::Option], // Using Option instead of Command
                key: "1".to_string(),
                global: true,
            },
            tiling_pattern_id: Some("three_column".to_string()),
            auto_arrange: true,
        };
        
        // This test should fail until workspace creation is implemented
        assert!(false, "Workspace creation not yet implemented");
    }

    #[test]
    fn test_switch_workspace_performance() {
        let request = SwitchWorkspaceRequest {
            workspace_id: "dev_workspace".to_string(),
            preserve_window_state: true,
        };
        
        // This test should fail until workspace switching is implemented
        // Performance requirement: <200ms switch time
        assert!(false, "Workspace switching not yet implemented");
    }

    #[test]
    fn test_duplicate_shortcut_detection() {
        // Should detect conflicts between Option+1 shortcuts
        let shortcut = KeyboardShortcut {
            modifiers: vec![ModifierKey::Option],
            key: "1".to_string(),
            global: true,
        };
        
        // This test should fail until duplicate detection is implemented
        assert!(false, "Shortcut conflict detection not yet implemented");
    }

    #[test]
    fn test_system_shortcut_conflict_prevention() {
        // Should prevent conflicts with system shortcuts
        // Example: Option+Space conflicts with Spotlight (if configured)
        let problematic_shortcut = KeyboardShortcut {
            modifiers: vec![ModifierKey::Option],
            key: "Space".to_string(),
            global: true,
        };
        
        // This test should fail until system conflict detection is implemented
        assert!(false, "System shortcut conflict prevention not yet implemented");
    }
}