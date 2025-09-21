use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// How a window should be positioned within a workspace
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PositioningRule {
    /// Follow workspace tiling pattern automatically
    Auto,
    /// Fixed position and size coordinates
    Fixed,
    /// Floating window (user-controlled position)
    Floating,
    /// Fullscreen mode
    Fullscreen,
}

/// Auto-focus behavior for windows
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FocusBehavior {
    /// Never auto-focus this window
    Never,
    /// Focus when window is created
    OnCreate,
    /// Focus when switching to this workspace
    OnSwitch,
}

/// Fixed position and size for windows with Fixed positioning rule
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FixedGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Defines how specific applications or window types behave within a workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowRule {
    /// Unique identifier
    pub id: Uuid,
    /// Reference to parent workspace
    pub workspace_id: Uuid,
    /// Bundle ID or process name pattern for application matching
    pub application_identifier: String,
    /// Optional regex pattern for matching window titles
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_title_pattern: Option<String>,
    /// How the window should be positioned
    pub positioning_rule: PositioningRule,
    /// Specific coordinates if positioning_rule is Fixed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed_geometry: Option<FixedGeometry>,
    /// Layer priority for window stacking (higher = more on top)
    pub z_order_priority: u32,
    /// Auto-focus behavior for this window
    pub focus_behavior: FocusBehavior,
    /// Optional reference to ApplicationProfile for default behavior
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application_profile_id: Option<Uuid>,
}

impl WindowRule {
    /// Create a new window rule with validation
    pub fn new(
        workspace_id: Uuid,
        application_identifier: String,
        window_title_pattern: Option<String>,
        positioning_rule: PositioningRule,
        fixed_geometry: Option<FixedGeometry>,
        z_order_priority: u32,
        focus_behavior: FocusBehavior,
        application_profile_id: Option<Uuid>,
    ) -> Result<Self, WindowRuleError> {
        let rule = WindowRule {
            id: Uuid::new_v4(),
            workspace_id,
            application_identifier,
            window_title_pattern,
            positioning_rule,
            fixed_geometry,
            z_order_priority,
            focus_behavior,
            application_profile_id,
        };

        rule.validate()?;
        Ok(rule)
    }

    /// Check if this rule matches a given application and window title
    pub fn matches(
        &self,
        app_identifier: &str,
        window_title: Option<&str>,
    ) -> Result<bool, WindowRuleError> {
        // Check application identifier match (exact or pattern)
        let app_matches = if self.application_identifier.contains('*')
            || self.application_identifier.contains('?')
        {
            // Simple glob-style matching
            self.glob_match(&self.application_identifier, app_identifier)
        } else {
            // Exact match
            self.application_identifier == app_identifier
        };

        if !app_matches {
            return Ok(false);
        }

        // Check window title pattern if specified
        if let Some(ref pattern) = self.window_title_pattern {
            if let Some(title) = window_title {
                let regex = Regex::new(pattern).map_err(|e| {
                    WindowRuleError::InvalidRegexPattern(pattern.clone(), e.to_string())
                })?;
                return Ok(regex.is_match(title));
            } else {
                // No window title provided but pattern is required
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Simple glob-style pattern matching for application identifiers
    fn glob_match(&self, pattern: &str, text: &str) -> bool {
        // Convert glob pattern to regex
        let regex_pattern = pattern
            .replace(".", "\\.")
            .replace("*", ".*")
            .replace("?", ".");

        if let Ok(regex) = Regex::new(&format!("^{}$", regex_pattern)) {
            regex.is_match(text)
        } else {
            // Fallback to exact match if regex creation fails
            pattern == text
        }
    }

    /// Get the effective geometry for this window rule
    pub fn get_effective_geometry(
        &self,
        default_geometry: Option<&FixedGeometry>,
    ) -> Option<FixedGeometry> {
        match self.positioning_rule {
            PositioningRule::Fixed => self
                .fixed_geometry
                .clone()
                .or_else(|| default_geometry.cloned()),
            _ => None,
        }
    }

    /// Check if this rule requires a specific z-order
    pub fn has_z_order_priority(&self) -> bool {
        self.z_order_priority > 0
    }

    /// Check if this rule should auto-focus in the given context
    pub fn should_auto_focus(&self, context: FocusContext) -> bool {
        match context {
            FocusContext::WindowCreated => self.focus_behavior == FocusBehavior::OnCreate,
            FocusContext::WorkspaceSwitch => self.focus_behavior == FocusBehavior::OnSwitch,
        }
    }

    /// Validate the window rule configuration
    pub fn validate(&self) -> Result<(), WindowRuleError> {
        // Validate application identifier
        if self.application_identifier.trim().is_empty() {
            return Err(WindowRuleError::EmptyApplicationIdentifier);
        }

        // Validate window title pattern if present
        if let Some(ref pattern) = self.window_title_pattern {
            if pattern.trim().is_empty() {
                return Err(WindowRuleError::EmptyWindowTitlePattern);
            }

            Regex::new(pattern).map_err(|e| {
                WindowRuleError::InvalidRegexPattern(pattern.clone(), e.to_string())
            })?;
        }

        // Validate fixed geometry if positioning rule is Fixed
        if self.positioning_rule == PositioningRule::Fixed && self.fixed_geometry.is_none() {
            return Err(WindowRuleError::MissingFixedGeometry);
        }

        // Validate fixed geometry bounds if present
        if let Some(ref geometry) = self.fixed_geometry {
            if geometry.width == 0 || geometry.height == 0 {
                return Err(WindowRuleError::InvalidGeometryDimensions);
            }
        }

        Ok(())
    }
}

/// Context for determining auto-focus behavior
#[derive(Debug, Clone, PartialEq)]
pub enum FocusContext {
    WindowCreated,
    WorkspaceSwitch,
}

/// Errors that can occur with window rules
#[derive(Debug, thiserror::Error)]
pub enum WindowRuleError {
    #[error("Application identifier cannot be empty")]
    EmptyApplicationIdentifier,

    #[error("Window title pattern cannot be empty")]
    EmptyWindowTitlePattern,

    #[error("Invalid regex pattern '{0}': {1}")]
    InvalidRegexPattern(String, String),

    #[error("Fixed positioning rule requires fixed geometry")]
    MissingFixedGeometry,

    #[error("Geometry dimensions must be greater than 0")]
    InvalidGeometryDimensions,

    #[error("Fixed geometry coordinates are outside screen bounds")]
    GeometryOutOfBounds,
}

impl Default for WindowRule {
    fn default() -> Self {
        WindowRule {
            id: Uuid::new_v4(),
            workspace_id: Uuid::new_v4(),
            application_identifier: "com.example.app".to_string(),
            window_title_pattern: None,
            positioning_rule: PositioningRule::Auto,
            fixed_geometry: None,
            z_order_priority: 0,
            focus_behavior: FocusBehavior::Never,
            application_profile_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_window_rule_valid() {
        let workspace_id = Uuid::new_v4();
        let rule = WindowRule::new(
            workspace_id,
            "com.apple.Terminal".to_string(),
            None,
            PositioningRule::Auto,
            None,
            0,
            FocusBehavior::OnCreate,
            None,
        );

        assert!(rule.is_ok());
        let rule = rule.unwrap();
        assert_eq!(rule.workspace_id, workspace_id);
        assert_eq!(rule.application_identifier, "com.apple.Terminal");
    }

    #[test]
    fn test_new_window_rule_invalid_empty_identifier() {
        let workspace_id = Uuid::new_v4();
        let rule = WindowRule::new(
            workspace_id,
            "".to_string(),
            None,
            PositioningRule::Auto,
            None,
            0,
            FocusBehavior::Never,
            None,
        );

        assert!(rule.is_err());
    }

    #[test]
    fn test_new_window_rule_fixed_missing_geometry() {
        let workspace_id = Uuid::new_v4();
        let rule = WindowRule::new(
            workspace_id,
            "com.example.app".to_string(),
            None,
            PositioningRule::Fixed,
            None, // Missing geometry for Fixed positioning
            0,
            FocusBehavior::Never,
            None,
        );

        assert!(rule.is_err());
    }

    #[test]
    fn test_matches_exact_application() {
        let rule = WindowRule {
            application_identifier: "com.apple.Terminal".to_string(),
            window_title_pattern: None,
            ..Default::default()
        };

        assert!(rule.matches("com.apple.Terminal", None).unwrap());
        assert!(!rule.matches("com.apple.Safari", None).unwrap());
    }

    #[test]
    fn test_matches_glob_pattern() {
        let rule = WindowRule {
            application_identifier: "com.apple.*".to_string(),
            window_title_pattern: None,
            ..Default::default()
        };

        assert!(rule.matches("com.apple.Terminal", None).unwrap());
        assert!(rule.matches("com.apple.Safari", None).unwrap());
        assert!(!rule.matches("com.google.Chrome", None).unwrap());
    }

    #[test]
    fn test_matches_window_title_pattern() {
        let rule = WindowRule {
            application_identifier: "com.apple.Terminal".to_string(),
            window_title_pattern: Some(r".*vim.*".to_string()),
            ..Default::default()
        };

        assert!(rule
            .matches("com.apple.Terminal", Some("editing file.txt - vim"))
            .unwrap());
        assert!(!rule
            .matches("com.apple.Terminal", Some("shell session"))
            .unwrap());
        assert!(!rule.matches("com.apple.Terminal", None).unwrap()); // No title provided
    }

    #[test]
    fn test_invalid_regex_pattern() {
        let rule = WindowRule {
            application_identifier: "com.apple.Terminal".to_string(),
            window_title_pattern: Some("[invalid regex".to_string()),
            ..Default::default()
        };

        assert!(rule
            .matches("com.apple.Terminal", Some("any title"))
            .is_err());
    }

    #[test]
    fn test_fixed_geometry() {
        let geometry = FixedGeometry {
            x: 100,
            y: 200,
            width: 800,
            height: 600,
        };

        let rule = WindowRule {
            positioning_rule: PositioningRule::Fixed,
            fixed_geometry: Some(geometry.clone()),
            ..Default::default()
        };

        assert_eq!(rule.get_effective_geometry(None), Some(geometry));
    }

    #[test]
    fn test_auto_focus_behavior() {
        let rule_on_create = WindowRule {
            focus_behavior: FocusBehavior::OnCreate,
            ..Default::default()
        };

        let rule_on_switch = WindowRule {
            focus_behavior: FocusBehavior::OnSwitch,
            ..Default::default()
        };

        let rule_never = WindowRule {
            focus_behavior: FocusBehavior::Never,
            ..Default::default()
        };

        assert!(rule_on_create.should_auto_focus(FocusContext::WindowCreated));
        assert!(!rule_on_create.should_auto_focus(FocusContext::WorkspaceSwitch));

        assert!(!rule_on_switch.should_auto_focus(FocusContext::WindowCreated));
        assert!(rule_on_switch.should_auto_focus(FocusContext::WorkspaceSwitch));

        assert!(!rule_never.should_auto_focus(FocusContext::WindowCreated));
        assert!(!rule_never.should_auto_focus(FocusContext::WorkspaceSwitch));
    }

    #[test]
    fn test_z_order_priority() {
        let rule_no_priority = WindowRule {
            z_order_priority: 0,
            ..Default::default()
        };

        let rule_with_priority = WindowRule {
            z_order_priority: 5,
            ..Default::default()
        };

        assert!(!rule_no_priority.has_z_order_priority());
        assert!(rule_with_priority.has_z_order_priority());
    }

    #[test]
    fn test_validation() {
        let mut rule = WindowRule::default();
        rule.application_identifier = "com.example.app".to_string();
        assert!(rule.validate().is_ok());

        // Test empty application identifier
        rule.application_identifier = "".to_string();
        assert!(rule.validate().is_err());

        // Test invalid regex pattern
        rule.application_identifier = "com.example.app".to_string();
        rule.window_title_pattern = Some("[invalid".to_string());
        assert!(rule.validate().is_err());

        // Test fixed positioning without geometry
        rule.window_title_pattern = None;
        rule.positioning_rule = PositioningRule::Fixed;
        rule.fixed_geometry = None;
        assert!(rule.validate().is_err());

        // Test invalid geometry dimensions
        rule.fixed_geometry = Some(FixedGeometry {
            x: 0,
            y: 0,
            width: 0, // Invalid
            height: 100,
        });
        assert!(rule.validate().is_err());
    }
}
