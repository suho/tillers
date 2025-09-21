use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Modifier keys for keyboard shortcuts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ModifierKey {
    /// Command key (⌘)
    Command,
    /// Option/Alt key (⌥)
    Option,
    /// Control key (⌃)
    Control,
    /// Shift key (⇧)
    Shift,
    /// Function key (fn)
    Function,
}

/// Regular keys for keyboard shortcuts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Key {
    /// Letter keys A-Z
    Letter(char),
    /// Number keys 0-9
    Number(u8),
    /// Function keys F1-F12
    Function(u8),
    /// Arrow keys
    Arrow(ArrowDirection),
    /// Special keys
    Space,
    Enter,
    Tab,
    Escape,
    Backspace,
    Delete,
    /// Custom key code
    KeyCode(u16),
}

/// Arrow key directions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ArrowDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Complete keyboard shortcut combination
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ShortcutCombination {
    /// Modifier keys that must be held
    pub modifiers: Vec<ModifierKey>,
    /// Primary key to press
    pub key: Key,
}

/// Types of actions that can be triggered by keyboard shortcuts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionType {
    /// Switch to a specific workspace
    SwitchWorkspace,
    /// Move current window to another workspace
    MoveWindow,
    /// Move current window to another monitor
    MoveWindowToMonitor,
    /// Resize current window
    ResizeWindow,
    /// Create a new workspace
    CreateWorkspace,
    /// Delete current workspace
    DeleteWorkspace,
    /// Toggle window floating mode
    ToggleFloating,
    /// Toggle window fullscreen mode
    ToggleFullscreen,
    /// Focus next window in workspace
    FocusNext,
    /// Focus previous window in workspace
    FocusPrevious,
    /// Close current window
    CloseWindow,
    /// Minimize current window
    MinimizeWindow,
    /// Refresh workspace layout
    RefreshLayout,
    /// Show workspace overview
    ShowOverview,
    /// Custom action with parameters
    Custom(String),
}

/// Resize directions for window resizing actions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResizeDirection {
    Increase,
    Decrease,
    IncreaseWidth,
    DecreaseWidth,
    IncreaseHeight,
    DecreaseHeight,
}

/// Parameters for different action types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionParameters {
    /// No parameters
    None,
    /// Target workspace ID for switch/move actions
    WorkspaceId(Uuid),
    /// Target monitor identifier for monitor move actions
    MonitorId(String),
    /// Resize direction and amount
    Resize(ResizeDirection, u32),
    /// Workspace name for creation
    WorkspaceName(String),
    /// Custom parameters as JSON
    Custom(serde_json::Value),
}

/// Defines keyboard shortcuts for workspace and window operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardMapping {
    /// Unique identifier
    pub id: Uuid,
    /// Human-readable name for this mapping
    pub name: String,
    /// Keyboard shortcut combination
    pub shortcut_combination: ShortcutCombination,
    /// Type of action to perform
    pub action_type: ActionType,
    /// Parameters for the action
    pub parameters: ActionParameters,
    /// Whether this shortcut is currently enabled
    pub enabled: bool,
    /// Whether shortcut works globally or only when app is focused
    pub global_scope: bool,
    /// Optional description of what this shortcut does
    pub description: Option<String>,
}

impl KeyboardMapping {
    /// Create a new keyboard mapping with validation
    pub fn new(
        name: String,
        shortcut_combination: ShortcutCombination,
        action_type: ActionType,
        parameters: ActionParameters,
        enabled: bool,
        global_scope: bool,
        description: Option<String>,
    ) -> Result<Self, KeyboardMappingError> {
        let mapping = KeyboardMapping {
            id: Uuid::new_v4(),
            name,
            shortcut_combination,
            action_type,
            parameters,
            enabled,
            global_scope,
            description,
        };

        mapping.validate()?;
        Ok(mapping)
    }

    /// Check if the action type matches the provided parameters
    pub fn action_parameters_match(&self) -> bool {
        match (&self.action_type, &self.parameters) {
            (ActionType::SwitchWorkspace, ActionParameters::WorkspaceId(_)) => true,
            (ActionType::SwitchWorkspace, ActionParameters::None) => true, // Allow None for defaults
            (ActionType::MoveWindow, ActionParameters::WorkspaceId(_)) => true,
            (ActionType::MoveWindowToMonitor, ActionParameters::MonitorId(_)) => true,
            (ActionType::ResizeWindow, ActionParameters::Resize(_, _)) => true,
            (ActionType::CreateWorkspace, ActionParameters::WorkspaceName(_)) => true,
            (ActionType::Custom(_), ActionParameters::Custom(_)) => true,
            (
                ActionType::DeleteWorkspace
                | ActionType::ToggleFloating
                | ActionType::ToggleFullscreen
                | ActionType::FocusNext
                | ActionType::FocusPrevious
                | ActionType::CloseWindow
                | ActionType::MinimizeWindow
                | ActionType::RefreshLayout
                | ActionType::ShowOverview,
                ActionParameters::None,
            ) => true,
            _ => false,
        }
    }

    /// Get a human-readable description of the shortcut
    pub fn get_description(&self) -> String {
        if let Some(ref desc) = self.description {
            desc.clone()
        } else {
            self.generate_description()
        }
    }

    /// Generate a default description based on action type and parameters
    fn generate_description(&self) -> String {
        match (&self.action_type, &self.parameters) {
            (ActionType::SwitchWorkspace, ActionParameters::WorkspaceId(_)) => {
                "Switch to workspace".to_string()
            }
            (ActionType::MoveWindow, ActionParameters::WorkspaceId(_)) => {
                "Move window to workspace".to_string()
            }
            (ActionType::MoveWindowToMonitor, ActionParameters::MonitorId(monitor)) => {
                format!("Move window to monitor {}", monitor)
            }
            (ActionType::ResizeWindow, ActionParameters::Resize(direction, amount)) => {
                format!("Resize window {:?} by {}", direction, amount)
            }
            (ActionType::CreateWorkspace, ActionParameters::WorkspaceName(name)) => {
                format!("Create workspace '{}'", name)
            }
            (ActionType::DeleteWorkspace, _) => "Delete current workspace".to_string(),
            (ActionType::ToggleFloating, _) => "Toggle window floating mode".to_string(),
            (ActionType::ToggleFullscreen, _) => "Toggle window fullscreen".to_string(),
            (ActionType::FocusNext, _) => "Focus next window".to_string(),
            (ActionType::FocusPrevious, _) => "Focus previous window".to_string(),
            (ActionType::CloseWindow, _) => "Close current window".to_string(),
            (ActionType::MinimizeWindow, _) => "Minimize current window".to_string(),
            (ActionType::RefreshLayout, _) => "Refresh workspace layout".to_string(),
            (ActionType::ShowOverview, _) => "Show workspace overview".to_string(),
            (ActionType::Custom(action), _) => format!("Custom action: {}", action),
            _ => "Unknown action".to_string(),
        }
    }

    /// Check if this mapping conflicts with another mapping
    pub fn conflicts_with(&self, other: &KeyboardMapping) -> bool {
        // Shortcuts conflict if they have the same combination and both are enabled
        self.enabled
            && other.enabled
            && self.shortcut_combination == other.shortcut_combination
            && self.id != other.id
    }

    /// Validate the keyboard mapping configuration
    pub fn validate(&self) -> Result<(), KeyboardMappingError> {
        // Validate name
        if self.name.trim().is_empty() {
            return Err(KeyboardMappingError::EmptyName);
        }

        // Validate shortcut combination
        self.shortcut_combination.validate()?;

        // Validate action parameters match
        if !self.action_parameters_match() {
            return Err(KeyboardMappingError::ActionParameterMismatch);
        }

        // Validate specific parameter types
        match &self.parameters {
            ActionParameters::Resize(_, amount) => {
                if *amount == 0 {
                    return Err(KeyboardMappingError::InvalidResizeAmount);
                }
            }
            ActionParameters::WorkspaceName(name) => {
                if name.trim().is_empty() {
                    return Err(KeyboardMappingError::EmptyWorkspaceName);
                }
            }
            ActionParameters::MonitorId(id) => {
                if id.trim().is_empty() {
                    return Err(KeyboardMappingError::EmptyMonitorId);
                }
            }
            _ => {}
        }

        Ok(())
    }
}

impl ShortcutCombination {
    /// Create a new shortcut combination
    pub fn new(modifiers: Vec<ModifierKey>, key: Key) -> Self {
        let mut sorted_modifiers = modifiers;
        sorted_modifiers.sort_by_key(|m| format!("{:?}", m));
        sorted_modifiers.dedup();

        ShortcutCombination {
            modifiers: sorted_modifiers,
            key,
        }
    }

    /// Validate the shortcut combination
    pub fn validate(&self) -> Result<(), KeyboardMappingError> {
        // Check for valid key
        match &self.key {
            Key::Letter(c) => {
                if !c.is_ascii_alphabetic() {
                    return Err(KeyboardMappingError::InvalidKey);
                }
            }
            Key::Number(n) => {
                if *n > 9 {
                    return Err(KeyboardMappingError::InvalidKey);
                }
            }
            Key::Function(f) => {
                if *f == 0 || *f > 12 {
                    return Err(KeyboardMappingError::InvalidKey);
                }
            }
            _ => {} // Other keys are valid
        }

        // Must have at least one modifier for global shortcuts
        if self.modifiers.is_empty() {
            return Err(KeyboardMappingError::NoModifiers);
        }

        Ok(())
    }
}

/// Collection of keyboard mappings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardMappingSet {
    pub mappings: Vec<KeyboardMapping>,
}

impl KeyboardMappingSet {
    /// Create a new empty keyboard mapping set
    pub fn new() -> Self {
        KeyboardMappingSet {
            mappings: Vec::new(),
        }
    }

    /// Add a keyboard mapping to the set
    pub fn add_mapping(&mut self, mapping: KeyboardMapping) -> Result<(), KeyboardMappingError> {
        // Check for conflicts with existing mappings
        for existing in &self.mappings {
            if mapping.conflicts_with(existing) {
                return Err(KeyboardMappingError::ConflictingShortcut(
                    mapping.shortcut_combination.to_string(),
                ));
            }
        }

        self.mappings.push(mapping);
        Ok(())
    }

    /// Remove a mapping by ID
    pub fn remove_mapping(&mut self, id: Uuid) -> bool {
        if let Some(pos) = self.mappings.iter().position(|m| m.id == id) {
            self.mappings.remove(pos);
            true
        } else {
            false
        }
    }

    /// Find a mapping by shortcut combination
    pub fn find_by_shortcut(&self, combination: &ShortcutCombination) -> Option<&KeyboardMapping> {
        self.mappings
            .iter()
            .find(|m| m.enabled && m.shortcut_combination == *combination)
    }

    /// Get all enabled global mappings
    pub fn get_global_mappings(&self) -> Vec<&KeyboardMapping> {
        self.mappings
            .iter()
            .filter(|m| m.enabled && m.global_scope)
            .collect()
    }

    /// Get all enabled local mappings
    pub fn get_local_mappings(&self) -> Vec<&KeyboardMapping> {
        self.mappings
            .iter()
            .filter(|m| m.enabled && !m.global_scope)
            .collect()
    }

    /// Create default keyboard mappings
    pub fn create_default() -> Result<Self, KeyboardMappingError> {
        let mut set = KeyboardMappingSet::new();

        // Default workspace switching shortcuts (Cmd+1 through Cmd+9)
        for i in 1..=9 {
            let mapping = KeyboardMapping::new(
                format!("Switch to Workspace {}", i),
                ShortcutCombination::new(vec![ModifierKey::Command], Key::Number(i)),
                ActionType::SwitchWorkspace,
                ActionParameters::None, // Will be filled in with actual workspace IDs
                true,
                true,
                Some(format!("Switch to workspace {}", i)),
            )?;
            set.add_mapping(mapping)?;
        }

        // Default window management shortcuts
        let default_mappings = vec![
            (
                "Focus Next Window",
                vec![ModifierKey::Command],
                Key::Tab,
                ActionType::FocusNext,
                ActionParameters::None,
            ),
            (
                "Focus Previous Window",
                vec![ModifierKey::Command, ModifierKey::Shift],
                Key::Tab,
                ActionType::FocusPrevious,
                ActionParameters::None,
            ),
            (
                "Close Window",
                vec![ModifierKey::Command],
                Key::Letter('w'),
                ActionType::CloseWindow,
                ActionParameters::None,
            ),
            (
                "Toggle Fullscreen",
                vec![ModifierKey::Command, ModifierKey::Control],
                Key::Letter('f'),
                ActionType::ToggleFullscreen,
                ActionParameters::None,
            ),
            (
                "Toggle Floating",
                vec![ModifierKey::Command, ModifierKey::Option],
                Key::Letter('f'),
                ActionType::ToggleFloating,
                ActionParameters::None,
            ),
        ];

        for (name, modifiers, key, action, params) in default_mappings {
            let mapping = KeyboardMapping::new(
                name.to_string(),
                ShortcutCombination::new(modifiers, key),
                action,
                params,
                true,
                true,
                None,
            )?;
            set.add_mapping(mapping)?;
        }

        Ok(set)
    }
}

impl fmt::Display for ShortcutCombination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let modifier_strs: Vec<String> = self
            .modifiers
            .iter()
            .map(|m| match m {
                ModifierKey::Command => "⌘".to_string(),
                ModifierKey::Option => "⌥".to_string(),
                ModifierKey::Control => "⌃".to_string(),
                ModifierKey::Shift => "⇧".to_string(),
                ModifierKey::Function => "fn".to_string(),
            })
            .collect();

        let key_str = match &self.key {
            Key::Letter(c) => c.to_uppercase().to_string(),
            Key::Number(n) => n.to_string(),
            Key::Function(f) => format!("F{}", f),
            Key::Arrow(direction) => match direction {
                ArrowDirection::Up => "↑".to_string(),
                ArrowDirection::Down => "↓".to_string(),
                ArrowDirection::Left => "←".to_string(),
                ArrowDirection::Right => "→".to_string(),
            },
            Key::Space => "Space".to_string(),
            Key::Enter => "Enter".to_string(),
            Key::Tab => "Tab".to_string(),
            Key::Escape => "Esc".to_string(),
            Key::Backspace => "⌫".to_string(),
            Key::Delete => "⌦".to_string(),
            Key::KeyCode(code) => format!("Key{}", code),
        };

        write!(f, "{}{}", modifier_strs.join(""), key_str)
    }
}

/// Errors that can occur with keyboard mappings
#[derive(Debug, thiserror::Error)]
pub enum KeyboardMappingError {
    #[error("Mapping name cannot be empty")]
    EmptyName,

    #[error("Invalid key specification")]
    InvalidKey,

    #[error("Shortcut must have at least one modifier key")]
    NoModifiers,

    #[error("Action type does not match provided parameters")]
    ActionParameterMismatch,

    #[error("Resize amount must be greater than 0")]
    InvalidResizeAmount,

    #[error("Workspace name cannot be empty")]
    EmptyWorkspaceName,

    #[error("Monitor ID cannot be empty")]
    EmptyMonitorId,

    #[error("Conflicting keyboard shortcut: {0}")]
    ConflictingShortcut(String),

    #[error("Invalid modifier key combination")]
    InvalidModifierCombination,
}

impl Default for KeyboardMapping {
    fn default() -> Self {
        KeyboardMapping {
            id: Uuid::new_v4(),
            name: "Default Mapping".to_string(),
            shortcut_combination: ShortcutCombination::new(
                vec![ModifierKey::Command],
                Key::Letter('a'),
            ),
            action_type: ActionType::ShowOverview,
            parameters: ActionParameters::None,
            enabled: true,
            global_scope: true,
            description: None,
        }
    }
}

impl Default for KeyboardMappingSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shortcut_combination_creation() {
        let combination = ShortcutCombination::new(
            vec![ModifierKey::Command, ModifierKey::Shift],
            Key::Letter('a'),
        );

        assert_eq!(combination.modifiers.len(), 2);
        assert!(combination.modifiers.contains(&ModifierKey::Command));
        assert!(combination.modifiers.contains(&ModifierKey::Shift));
        assert_eq!(combination.key, Key::Letter('a'));
    }

    #[test]
    fn test_shortcut_combination_deduplication() {
        let combination = ShortcutCombination::new(
            vec![ModifierKey::Command, ModifierKey::Command, ModifierKey::Shift],
            Key::Letter('a'),
        );

        assert_eq!(combination.modifiers.len(), 2);
    }

    #[test]
    fn test_keyboard_mapping_creation() {
        let combination = ShortcutCombination::new(vec![ModifierKey::Command], Key::Number(1));
        let mapping = KeyboardMapping::new(
            "Test Mapping".to_string(),
            combination,
            ActionType::SwitchWorkspace,
            ActionParameters::WorkspaceId(Uuid::new_v4()),
            true,
            true,
            None,
        );

        assert!(mapping.is_ok());
        let mapping = mapping.unwrap();
        assert_eq!(mapping.name, "Test Mapping");
        assert!(mapping.enabled);
        assert!(mapping.global_scope);
    }

    #[test]
    fn test_keyboard_mapping_parameter_validation() {
        let combination = ShortcutCombination::new(vec![ModifierKey::Command], Key::Number(1));
        
        // Valid parameter match
        let valid_mapping = KeyboardMapping::new(
            "Test".to_string(),
            combination.clone(),
            ActionType::SwitchWorkspace,
            ActionParameters::WorkspaceId(Uuid::new_v4()),
            true,
            true,
            None,
        );
        assert!(valid_mapping.is_ok());

        // Invalid parameter match - use ResizeWindow with wrong params
        let invalid_mapping = KeyboardMapping::new(
            "Test".to_string(),
            combination,
            ActionType::ResizeWindow,
            ActionParameters::None, // Wrong parameter type for ResizeWindow
            true,
            true,
            None,
        );
        assert!(invalid_mapping.is_err());
    }

    #[test]
    fn test_keyboard_mapping_conflicts() {
        let combination = ShortcutCombination::new(vec![ModifierKey::Command], Key::Letter('a'));
        
        let mapping1 = KeyboardMapping {
            shortcut_combination: combination.clone(),
            enabled: true,
            ..Default::default()
        };

        let mapping2 = KeyboardMapping {
            shortcut_combination: combination,
            enabled: true,
            ..Default::default()
        };

        assert!(mapping1.conflicts_with(&mapping2));
        assert!(mapping2.conflicts_with(&mapping1));
    }

    #[test]
    fn test_keyboard_mapping_set() {
        let mut set = KeyboardMappingSet::new();
        
        let mapping1 = KeyboardMapping {
            shortcut_combination: ShortcutCombination::new(
                vec![ModifierKey::Command],
                Key::Letter('a'),
            ),
            enabled: true,
            ..Default::default()
        };

        let mapping2 = KeyboardMapping {
            shortcut_combination: ShortcutCombination::new(
                vec![ModifierKey::Command],
                Key::Letter('b'),
            ),
            enabled: true,
            ..Default::default()
        };

        assert!(set.add_mapping(mapping1.clone()).is_ok());
        assert!(set.add_mapping(mapping2).is_ok());

        // Test conflict detection
        let conflicting_mapping = KeyboardMapping {
            shortcut_combination: ShortcutCombination::new(
                vec![ModifierKey::Command],
                Key::Letter('a'),
            ),
            enabled: true,
            ..Default::default()
        };

        assert!(set.add_mapping(conflicting_mapping).is_err());

        // Test finding by shortcut
        let found = set.find_by_shortcut(&mapping1.shortcut_combination);
        assert!(found.is_some());
    }

    #[test]
    fn test_shortcut_display() {
        let combination = ShortcutCombination::new(
            vec![ModifierKey::Command, ModifierKey::Shift],
            Key::Letter('a'),
        );

        let display = format!("{}", combination);
        assert!(display.contains("⌘"));
        assert!(display.contains("⇧"));
        assert!(display.contains("A"));
    }

    #[test]
    fn test_default_mappings_creation() {
        let set = KeyboardMappingSet::create_default();
        assert!(set.is_ok());
        
        let set = set.unwrap();
        assert!(!set.mappings.is_empty());
        
        // Check that we have workspace switching shortcuts
        let workspace_shortcuts = set.mappings.iter()
            .filter(|m| matches!(m.action_type, ActionType::SwitchWorkspace))
            .count();
        assert_eq!(workspace_shortcuts, 9); // Cmd+1 through Cmd+9
    }

    #[test]
    fn test_action_description_generation() {
        let mapping = KeyboardMapping {
            action_type: ActionType::ToggleFullscreen,
            parameters: ActionParameters::None,
            description: None,
            ..Default::default()
        };

        let description = mapping.get_description();
        assert_eq!(description, "Toggle window fullscreen");
    }

    #[test]
    fn test_validation_errors() {
        // Test empty name
        let mut mapping = KeyboardMapping::default();
        mapping.name = "".to_string();
        assert!(mapping.validate().is_err());

        // Test no modifiers
        mapping.name = "Test".to_string();
        mapping.shortcut_combination.modifiers.clear();
        assert!(mapping.validate().is_err());

        // Test invalid resize amount
        mapping.shortcut_combination.modifiers.push(ModifierKey::Command);
        mapping.action_type = ActionType::ResizeWindow;
        mapping.parameters = ActionParameters::Resize(ResizeDirection::Increase, 0);
        assert!(mapping.validate().is_err());
    }
}