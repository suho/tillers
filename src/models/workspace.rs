//! Workspace model for TilleRS window manager
//! 
//! Represents a logical grouping of applications and windows with defined layout rules

use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{Result, TilleRSError};

/// Workspace state for tracking active/inactive status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WorkspaceState {
    #[default]
    Inactive,
    Active,
    Modified,
}

/// Workspace represents a logical grouping of applications and windows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    /// Unique identifier
    pub id: Uuid,
    
    /// Human-readable workspace name
    pub name: String,
    
    /// Optional workspace description
    pub description: Option<String>,
    
    /// Key combination for switching to this workspace
    pub keyboard_shortcut: String,
    
    /// Reference to default tiling pattern
    pub tiling_pattern_id: Uuid,
    
    /// Map of monitor IDs to layout preferences
    pub monitor_assignments: HashMap<String, String>,
    
    /// Boolean flag for automatic window arrangement
    pub auto_arrange: bool,
    
    /// Timestamp of workspace creation
    pub created_at: DateTime<Utc>,
    
    /// Timestamp of last activation
    pub last_used: Option<DateTime<Utc>>,
    
    /// Current workspace state
    #[serde(skip)]
    pub state: WorkspaceState,
}

/// Request structure for creating a new workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceCreateRequest {
    /// Human-readable workspace name
    pub name: String,
    
    /// Optional workspace description
    pub description: Option<String>,
    
    /// Key combination for switching to this workspace
    pub keyboard_shortcut: String,
    
    /// Reference to default tiling pattern (optional, uses default if not provided)
    pub tiling_pattern_id: Option<Uuid>,
    
    /// Boolean flag for automatic window arrangement
    pub auto_arrange: Option<bool>,
}

/// Request structure for updating an existing workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceUpdateRequest {
    /// Human-readable workspace name
    pub name: Option<String>,
    
    /// Optional workspace description
    pub description: Option<String>,
    
    /// Key combination for switching to this workspace
    pub keyboard_shortcut: Option<String>,
    
    /// Reference to default tiling pattern
    pub tiling_pattern_id: Option<Uuid>,
    
    /// Boolean flag for automatic window arrangement
    pub auto_arrange: Option<bool>,
}

impl Workspace {
    /// Create a new workspace from a creation request
    pub fn new(request: WorkspaceCreateRequest, default_pattern_id: Uuid) -> Result<Self> {
        // Validate the request
        Self::validate_create_request(&request)?;
        
        let now = Utc::now();
        let pattern_id = request.tiling_pattern_id.unwrap_or(default_pattern_id);
        
        Ok(Workspace {
            id: Uuid::new_v4(),
            name: request.name,
            description: request.description,
            keyboard_shortcut: request.keyboard_shortcut,
            tiling_pattern_id: pattern_id,
            monitor_assignments: HashMap::new(),
            auto_arrange: request.auto_arrange.unwrap_or(true),
            created_at: now,
            last_used: None,
            state: WorkspaceState::Inactive,
        })
    }
    
    /// Update workspace with provided changes
    pub fn update(&mut self, request: WorkspaceUpdateRequest) -> Result<()> {
        // Validate the update request
        Self::validate_update_request(&request)?;
        
        if let Some(name) = request.name {
            self.name = name;
        }
        
        if let Some(description) = request.description {
            self.description = Some(description);
        }
        
        if let Some(keyboard_shortcut) = request.keyboard_shortcut {
            self.keyboard_shortcut = keyboard_shortcut;
        }
        
        if let Some(tiling_pattern_id) = request.tiling_pattern_id {
            self.tiling_pattern_id = tiling_pattern_id;
        }
        
        if let Some(auto_arrange) = request.auto_arrange {
            self.auto_arrange = auto_arrange;
        }
        
        Ok(())
    }
    
    /// Activate this workspace
    pub fn activate(&mut self) {
        self.state = WorkspaceState::Active;
        self.last_used = Some(Utc::now());
    }
    
    /// Deactivate this workspace
    pub fn deactivate(&mut self) {
        if self.state == WorkspaceState::Active || self.state == WorkspaceState::Modified {
            self.state = WorkspaceState::Inactive;
        }
    }
    
    /// Mark workspace as modified (layout changed)
    pub fn mark_modified(&mut self) {
        if self.state == WorkspaceState::Active {
            self.state = WorkspaceState::Modified;
        }
    }
    
    /// Check if workspace is currently active
    pub fn is_active(&self) -> bool {
        self.state == WorkspaceState::Active
    }
    
    /// Validate workspace creation request
    fn validate_create_request(request: &WorkspaceCreateRequest) -> Result<()> {
        // Validate name
        if request.name.trim().is_empty() {
            return Err(TilleRSError::ValidationError(
                "Workspace name cannot be empty".to_string()
            ).into());
        }
        
        if request.name.len() > 100 {
            return Err(TilleRSError::ValidationError(
                "Workspace name cannot exceed 100 characters".to_string()
            ).into());
        }
        
        // Validate keyboard shortcut format
        Self::validate_keyboard_shortcut(&request.keyboard_shortcut)?;
        
        // Validate description length if provided
        if let Some(ref desc) = request.description {
            if desc.len() > 500 {
                return Err(TilleRSError::ValidationError(
                    "Workspace description cannot exceed 500 characters".to_string()
                ).into());
            }
        }
        
        Ok(())
    }
    
    /// Validate workspace update request
    fn validate_update_request(request: &WorkspaceUpdateRequest) -> Result<()> {
        // Validate name if provided
        if let Some(ref name) = request.name {
            if name.trim().is_empty() {
                return Err(TilleRSError::ValidationError(
                    "Workspace name cannot be empty".to_string()
                ).into());
            }
            
            if name.len() > 100 {
                return Err(TilleRSError::ValidationError(
                    "Workspace name cannot exceed 100 characters".to_string()
                ).into());
            }
        }
        
        // Validate keyboard shortcut if provided
        if let Some(ref shortcut) = request.keyboard_shortcut {
            Self::validate_keyboard_shortcut(shortcut)?;
        }
        
        // Validate description length if provided
        if let Some(ref desc) = request.description {
            if desc.len() > 500 {
                return Err(TilleRSError::ValidationError(
                    "Workspace description cannot exceed 500 characters".to_string()
                ).into());
            }
        }
        
        Ok(())
    }
    
    /// Validate keyboard shortcut format
    fn validate_keyboard_shortcut(shortcut: &str) -> Result<()> {
        // Basic validation for keyboard shortcut format
        // Should match pattern: (cmd|ctrl|opt|shift)(+(cmd|ctrl|opt|shift))*+[a-zA-Z0-9]
        
        if shortcut.is_empty() {
            return Err(TilleRSError::ValidationError(
                "Keyboard shortcut cannot be empty".to_string()
            ).into());
        }
        
        let parts: Vec<&str> = shortcut.split('+').collect();
        if parts.len() < 2 {
            return Err(TilleRSError::ValidationError(
                "Keyboard shortcut must include at least one modifier and one key".to_string()
            ).into());
        }
        
        // Check modifiers (all parts except the last one)
        let modifiers = &parts[..parts.len() - 1];
        for modifier in modifiers {
            if !["cmd", "ctrl", "opt", "shift"].contains(modifier) {
                return Err(TilleRSError::ValidationError(
                    format!("Invalid modifier '{}' in keyboard shortcut", modifier)
                ).into());
            }
        }
        
        // Check key (last part)
        let key = parts[parts.len() - 1];
        if key.is_empty() || (!key.chars().all(|c| c.is_alphanumeric()) && !key.starts_with('F')) {
            return Err(TilleRSError::ValidationError(
                format!("Invalid key '{}' in keyboard shortcut", key)
            ).into());
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_creation() {
        let request = WorkspaceCreateRequest {
            name: "Test Workspace".to_string(),
            description: Some("A test workspace".to_string()),
            keyboard_shortcut: "cmd+1".to_string(),
            tiling_pattern_id: None,
            auto_arrange: Some(true),
        };
        
        let default_pattern = Uuid::new_v4();
        let workspace = Workspace::new(request, default_pattern).unwrap();
        
        assert_eq!(workspace.name, "Test Workspace");
        assert_eq!(workspace.description, Some("A test workspace".to_string()));
        assert_eq!(workspace.keyboard_shortcut, "cmd+1");
        assert_eq!(workspace.tiling_pattern_id, default_pattern);
        assert!(workspace.auto_arrange);
        assert_eq!(workspace.state, WorkspaceState::Inactive);
    }
    
    #[test]
    fn test_workspace_validation() {
        // Test empty name
        let invalid_request = WorkspaceCreateRequest {
            name: "".to_string(),
            description: None,
            keyboard_shortcut: "cmd+1".to_string(),
            tiling_pattern_id: None,
            auto_arrange: None,
        };
        
        let result = Workspace::new(invalid_request, Uuid::new_v4());
        assert!(result.is_err());
        
        // Test invalid keyboard shortcut
        let invalid_request = WorkspaceCreateRequest {
            name: "Valid Name".to_string(),
            description: None,
            keyboard_shortcut: "invalid".to_string(),
            tiling_pattern_id: None,
            auto_arrange: None,
        };
        
        let result = Workspace::new(invalid_request, Uuid::new_v4());
        assert!(result.is_err());
    }
    
    #[test]
    fn test_workspace_state_transitions() {
        let request = WorkspaceCreateRequest {
            name: "Test".to_string(),
            description: None,
            keyboard_shortcut: "cmd+1".to_string(),
            tiling_pattern_id: None,
            auto_arrange: None,
        };
        
        let mut workspace = Workspace::new(request, Uuid::new_v4()).unwrap();
        
        // Initial state should be Inactive
        assert_eq!(workspace.state, WorkspaceState::Inactive);
        assert!(!workspace.is_active());
        
        // Activate workspace
        workspace.activate();
        assert_eq!(workspace.state, WorkspaceState::Active);
        assert!(workspace.is_active());
        assert!(workspace.last_used.is_some());
        
        // Mark as modified
        workspace.mark_modified();
        assert_eq!(workspace.state, WorkspaceState::Modified);
        
        // Deactivate workspace
        workspace.deactivate();
        assert_eq!(workspace.state, WorkspaceState::Inactive);
        assert!(!workspace.is_active());
    }
    
    #[test]
    fn test_keyboard_shortcut_validation() {
        // Valid shortcuts
        assert!(Workspace::validate_keyboard_shortcut("cmd+1").is_ok());
        assert!(Workspace::validate_keyboard_shortcut("cmd+shift+a").is_ok());
        assert!(Workspace::validate_keyboard_shortcut("ctrl+opt+F1").is_ok());
        
        // Invalid shortcuts
        assert!(Workspace::validate_keyboard_shortcut("").is_err());
        assert!(Workspace::validate_keyboard_shortcut("cmd").is_err());
        assert!(Workspace::validate_keyboard_shortcut("invalid+1").is_err());
        assert!(Workspace::validate_keyboard_shortcut("cmd+").is_err());
    }
}