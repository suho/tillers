use crate::models::{FocusBehavior, PositioningRule};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Application window behavior characteristics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WindowBehavior {
    /// Standard window that follows tiling rules
    Standard,
    /// Always stays on top of other windows
    AlwaysOnTop,
    /// Cannot be tiled (e.g., system dialogs)
    NonTileable,
    /// Transient window (tooltips, pop-ups)
    Transient,
    /// Modal dialog
    Modal,
    /// Utility window (palette, toolbar)
    Utility,
}

/// How the application handles focus changes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FocusStealingBehavior {
    /// Normal focus behavior
    Normal,
    /// Application steals focus aggressively
    Aggressive,
    /// Application never steals focus
    Passive,
    /// Application focuses new windows but not existing ones
    NewWindowsOnly,
}

/// Window detection strategies for identifying app windows
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WindowDetectionStrategy {
    /// Use bundle identifier only
    BundleId,
    /// Use process name
    ProcessName,
    /// Use window class information
    WindowClass,
    /// Use combination of multiple strategies
    Combined,
}

/// Rules for detecting windows belonging to this application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowDetectionRule {
    /// Detection strategy to use
    pub strategy: WindowDetectionStrategy,
    /// Primary identifier (bundle ID, process name, etc.)
    pub primary_identifier: String,
    /// Secondary identifiers for Combined strategy
    pub secondary_identifiers: Vec<String>,
    /// Regex pattern for window titles (optional)
    pub title_pattern: Option<String>,
    /// Regex pattern for window class names (optional)
    pub class_pattern: Option<String>,
}

/// Compatibility notes and known issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityInfo {
    /// Known compatibility level
    pub level: CompatibilityLevel,
    /// Specific issues or limitations
    pub issues: Vec<String>,
    /// Workarounds or special handling required
    pub workarounds: Vec<String>,
    /// Version information if compatibility is version-specific
    pub version_notes: Option<String>,
}

/// Compatibility levels for applications
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CompatibilityLevel {
    /// Fully compatible, all features work
    Full,
    /// Mostly compatible with minor issues
    Good,
    /// Basic compatibility with some limitations
    Limited,
    /// Compatibility issues, may not work properly
    Poor,
    /// Incompatible with tiling window manager
    Incompatible,
}

/// Custom handling rules for specific application scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomHandlingRule {
    /// Condition that triggers this rule
    pub condition: String,
    /// Action to take when condition is met
    pub action: CustomAction,
    /// Optional parameters for the action
    pub parameters: Option<serde_json::Value>,
    /// Whether this rule is currently active
    pub enabled: bool,
}

/// Custom actions for application-specific handling
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CustomAction {
    /// Ignore the window entirely
    Ignore,
    /// Force specific positioning rule
    ForcePositioning(PositioningRule),
    /// Delay window management for specified milliseconds
    DelayManagement(u32),
    /// Use custom window detection logic
    CustomDetection,
    /// Apply special focus handling
    SpecialFocus,
    /// Run custom script or command
    RunScript(String),
}

/// Stores default behavior patterns for specific applications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationProfile {
    /// Unique identifier
    pub id: Uuid,
    /// macOS bundle identifier (e.g., com.apple.Terminal)
    pub bundle_identifier: String,
    /// Human-readable application name
    pub display_name: String,
    /// Default positioning rule for new windows
    pub default_positioning: PositioningRule,
    /// Preferred tiling patterns (in order of preference)
    pub preferred_tiling_patterns: Vec<Uuid>,
    /// Compatibility information and known issues
    pub compatibility_info: CompatibilityInfo,
    /// Rules for identifying application windows
    pub window_detection_rules: Vec<WindowDetectionRule>,
    /// How the application handles focus changes
    pub focus_stealing_behavior: FocusStealingBehavior,
    /// Default window behavior characteristics
    pub default_window_behavior: WindowBehavior,
    /// Default focus behavior for windows
    pub default_focus_behavior: FocusBehavior,
    /// Custom handling rules for special scenarios
    pub custom_rules: Vec<CustomHandlingRule>,
    /// Application-specific configuration overrides
    pub configuration_overrides: HashMap<String, serde_json::Value>,
    /// Whether this profile is user-created or built-in
    pub is_user_profile: bool,
    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl ApplicationProfile {
    /// Create a new application profile with validation
    pub fn new(
        bundle_identifier: String,
        display_name: String,
        default_positioning: PositioningRule,
        compatibility_level: CompatibilityLevel,
    ) -> Result<Self, ApplicationProfileError> {
        let profile = ApplicationProfile {
            id: Uuid::new_v4(),
            bundle_identifier,
            display_name,
            default_positioning,
            preferred_tiling_patterns: Vec::new(),
            compatibility_info: CompatibilityInfo {
                level: compatibility_level,
                issues: Vec::new(),
                workarounds: Vec::new(),
                version_notes: None,
            },
            window_detection_rules: Vec::new(),
            focus_stealing_behavior: FocusStealingBehavior::Normal,
            default_window_behavior: WindowBehavior::Standard,
            default_focus_behavior: FocusBehavior::Never,
            custom_rules: Vec::new(),
            configuration_overrides: HashMap::new(),
            is_user_profile: true,
            last_updated: chrono::Utc::now(),
        };

        profile.validate()?;
        Ok(profile)
    }

    /// Check if this profile matches a given application
    pub fn matches_application(
        &self,
        bundle_id: Option<&str>,
        process_name: Option<&str>,
        window_title: Option<&str>,
    ) -> bool {
        // Primary match by bundle identifier
        if let Some(bundle_id) = bundle_id {
            if self.bundle_identifier == bundle_id {
                return true;
            }
        }

        // Secondary match using detection rules
        for rule in &self.window_detection_rules {
            if self.matches_detection_rule(rule, bundle_id, process_name, window_title) {
                return true;
            }
        }

        false
    }

    /// Check if application matches a specific detection rule
    fn matches_detection_rule(
        &self,
        rule: &WindowDetectionRule,
        bundle_id: Option<&str>,
        process_name: Option<&str>,
        _window_title: Option<&str>,
    ) -> bool {
        match rule.strategy {
            WindowDetectionStrategy::BundleId => {
                bundle_id.map_or(false, |id| id == rule.primary_identifier)
            }
            WindowDetectionStrategy::ProcessName => {
                process_name.map_or(false, |name| name == rule.primary_identifier)
            }
            WindowDetectionStrategy::WindowClass => {
                // Window class matching would need additional system info
                // For now, fall back to bundle ID
                bundle_id.map_or(false, |id| id == rule.primary_identifier)
            }
            WindowDetectionStrategy::Combined => {
                let primary_match = bundle_id.map_or(false, |id| id == rule.primary_identifier)
                    || process_name.map_or(false, |name| name == rule.primary_identifier);

                let secondary_match = rule.secondary_identifiers.iter().any(|identifier| {
                    bundle_id.map_or(false, |id| id == identifier)
                        || process_name.map_or(false, |name| name == identifier)
                });

                primary_match || secondary_match
            }
        }
    }

    /// Get the effective positioning rule for a window
    pub fn get_effective_positioning(&self, window_title: Option<&str>) -> PositioningRule {
        // Check custom rules first
        for rule in &self.custom_rules {
            if rule.enabled {
                if let CustomAction::ForcePositioning(positioning) = &rule.action {
                    // Simple condition matching for now
                    if window_title.map_or(false, |title| title.contains(&rule.condition)) {
                        return positioning.clone();
                    }
                }
            }
        }

        // Return default positioning
        self.default_positioning.clone()
    }

    /// Check if the application is compatible with tiling
    pub fn is_tiling_compatible(&self) -> bool {
        matches!(
            self.compatibility_info.level,
            CompatibilityLevel::Full | CompatibilityLevel::Good | CompatibilityLevel::Limited
        )
    }

    /// Get custom handling action for a specific condition
    pub fn get_custom_action(&self, condition: &str) -> Option<&CustomAction> {
        self.custom_rules
            .iter()
            .find(|rule| rule.enabled && rule.condition == condition)
            .map(|rule| &rule.action)
    }

    /// Add a preferred tiling pattern
    pub fn add_preferred_pattern(&mut self, pattern_id: Uuid) {
        if !self.preferred_tiling_patterns.contains(&pattern_id) {
            self.preferred_tiling_patterns.push(pattern_id);
            self.last_updated = chrono::Utc::now();
        }
    }

    /// Remove a preferred tiling pattern
    pub fn remove_preferred_pattern(&mut self, pattern_id: &Uuid) -> bool {
        if let Some(pos) = self
            .preferred_tiling_patterns
            .iter()
            .position(|id| id == pattern_id)
        {
            self.preferred_tiling_patterns.remove(pos);
            self.last_updated = chrono::Utc::now();
            true
        } else {
            false
        }
    }

    /// Add a custom handling rule
    pub fn add_custom_rule(&mut self, rule: CustomHandlingRule) {
        self.custom_rules.push(rule);
        self.last_updated = chrono::Utc::now();
    }

    /// Update compatibility information
    pub fn update_compatibility(&mut self, info: CompatibilityInfo) {
        self.compatibility_info = info;
        self.last_updated = chrono::Utc::now();
    }

    /// Validate the application profile
    pub fn validate(&self) -> Result<(), ApplicationProfileError> {
        // Validate bundle identifier
        if self.bundle_identifier.trim().is_empty() {
            return Err(ApplicationProfileError::EmptyBundleIdentifier);
        }

        // Bundle identifier should follow macOS convention
        if !self.bundle_identifier.contains('.') {
            return Err(ApplicationProfileError::InvalidBundleIdentifier(
                self.bundle_identifier.clone(),
            ));
        }

        // Validate display name
        if self.display_name.trim().is_empty() {
            return Err(ApplicationProfileError::EmptyDisplayName);
        }

        // Validate detection rules
        for rule in &self.window_detection_rules {
            if rule.primary_identifier.trim().is_empty() {
                return Err(ApplicationProfileError::EmptyDetectionIdentifier);
            }
        }

        // Validate custom rules
        for rule in &self.custom_rules {
            if rule.condition.trim().is_empty() {
                return Err(ApplicationProfileError::EmptyCustomRuleCondition);
            }
        }

        Ok(())
    }
}

/// Collection of application profiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationProfileSet {
    pub profiles: Vec<ApplicationProfile>,
}

impl ApplicationProfileSet {
    /// Create a new empty application profile set
    pub fn new() -> Self {
        ApplicationProfileSet {
            profiles: Vec::new(),
        }
    }

    /// Add an application profile
    pub fn add_profile(
        &mut self,
        profile: ApplicationProfile,
    ) -> Result<(), ApplicationProfileError> {
        // Check for duplicate bundle identifiers
        if self
            .profiles
            .iter()
            .any(|p| p.bundle_identifier == profile.bundle_identifier)
        {
            return Err(ApplicationProfileError::DuplicateBundleIdentifier(
                profile.bundle_identifier,
            ));
        }

        self.profiles.push(profile);
        Ok(())
    }

    /// Find a profile by bundle identifier
    pub fn find_by_bundle_id(&self, bundle_id: &str) -> Option<&ApplicationProfile> {
        self.profiles
            .iter()
            .find(|profile| profile.bundle_identifier == bundle_id)
    }

    /// Find a profile that matches the given application characteristics
    pub fn find_matching_profile(
        &self,
        bundle_id: Option<&str>,
        process_name: Option<&str>,
        window_title: Option<&str>,
    ) -> Option<&ApplicationProfile> {
        self.profiles
            .iter()
            .find(|profile| profile.matches_application(bundle_id, process_name, window_title))
    }

    /// Get all profiles with specific compatibility level
    pub fn get_profiles_by_compatibility(
        &self,
        level: CompatibilityLevel,
    ) -> Vec<&ApplicationProfile> {
        self.profiles
            .iter()
            .filter(|profile| profile.compatibility_info.level == level)
            .collect()
    }

    /// Remove a profile by ID
    pub fn remove_profile(&mut self, id: Uuid) -> bool {
        if let Some(pos) = self.profiles.iter().position(|p| p.id == id) {
            self.profiles.remove(pos);
            true
        } else {
            false
        }
    }

    /// Create a set with common macOS application profiles
    pub fn create_with_defaults() -> Result<Self, ApplicationProfileError> {
        let mut set = ApplicationProfileSet::new();

        // Common macOS applications with known profiles
        let default_profiles = vec![
            (
                "com.apple.Terminal",
                "Terminal",
                PositioningRule::Auto,
                CompatibilityLevel::Full,
            ),
            (
                "com.apple.Safari",
                "Safari",
                PositioningRule::Auto,
                CompatibilityLevel::Full,
            ),
            (
                "com.microsoft.VSCode",
                "Visual Studio Code",
                PositioningRule::Auto,
                CompatibilityLevel::Full,
            ),
            (
                "com.apple.finder",
                "Finder",
                PositioningRule::Floating,
                CompatibilityLevel::Good,
            ),
            (
                "com.adobe.Photoshop",
                "Adobe Photoshop",
                PositioningRule::Auto,
                CompatibilityLevel::Limited,
            ),
            (
                "com.apple.systempreferences",
                "System Preferences",
                PositioningRule::Floating,
                CompatibilityLevel::Poor,
            ),
        ];

        for (bundle_id, display_name, positioning, compatibility) in default_profiles {
            let mut profile = ApplicationProfile::new(
                bundle_id.to_string(),
                display_name.to_string(),
                positioning,
                compatibility,
            )?;

            profile.is_user_profile = false; // Built-in profile

            // Add default detection rule
            profile.window_detection_rules.push(WindowDetectionRule {
                strategy: WindowDetectionStrategy::BundleId,
                primary_identifier: bundle_id.to_string(),
                secondary_identifiers: Vec::new(),
                title_pattern: None,
                class_pattern: None,
            });

            set.add_profile(profile)?;
        }

        Ok(set)
    }
}

/// Errors that can occur with application profiles
#[derive(Debug, thiserror::Error)]
pub enum ApplicationProfileError {
    #[error("Bundle identifier cannot be empty")]
    EmptyBundleIdentifier,

    #[error("Invalid bundle identifier format: {0}")]
    InvalidBundleIdentifier(String),

    #[error("Display name cannot be empty")]
    EmptyDisplayName,

    #[error("Detection identifier cannot be empty")]
    EmptyDetectionIdentifier,

    #[error("Custom rule condition cannot be empty")]
    EmptyCustomRuleCondition,

    #[error("Duplicate bundle identifier: {0}")]
    DuplicateBundleIdentifier(String),

    #[error("Profile not found for bundle ID: {0}")]
    ProfileNotFound(String),

    #[error("Invalid custom action parameters")]
    InvalidCustomActionParameters,
}

impl Default for ApplicationProfile {
    fn default() -> Self {
        ApplicationProfile {
            id: Uuid::new_v4(),
            bundle_identifier: "com.example.app".to_string(),
            display_name: "Example App".to_string(),
            default_positioning: PositioningRule::Auto,
            preferred_tiling_patterns: Vec::new(),
            compatibility_info: CompatibilityInfo {
                level: CompatibilityLevel::Full,
                issues: Vec::new(),
                workarounds: Vec::new(),
                version_notes: None,
            },
            window_detection_rules: Vec::new(),
            focus_stealing_behavior: FocusStealingBehavior::Normal,
            default_window_behavior: WindowBehavior::Standard,
            default_focus_behavior: FocusBehavior::Never,
            custom_rules: Vec::new(),
            configuration_overrides: HashMap::new(),
            is_user_profile: true,
            last_updated: chrono::Utc::now(),
        }
    }
}

impl Default for ApplicationProfileSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_application_profile() {
        let profile = ApplicationProfile::new(
            "com.apple.Terminal".to_string(),
            "Terminal".to_string(),
            PositioningRule::Auto,
            CompatibilityLevel::Full,
        );

        assert!(profile.is_ok());
        let profile = profile.unwrap();
        assert_eq!(profile.bundle_identifier, "com.apple.Terminal");
        assert_eq!(profile.display_name, "Terminal");
        assert!(profile.is_user_profile);
    }

    #[test]
    fn test_invalid_bundle_identifier() {
        let profile = ApplicationProfile::new(
            "invalid".to_string(), // No dot
            "App".to_string(),
            PositioningRule::Auto,
            CompatibilityLevel::Full,
        );

        assert!(profile.is_err());
    }

    #[test]
    fn test_application_matching() {
        let profile = ApplicationProfile {
            bundle_identifier: "com.apple.Terminal".to_string(),
            ..Default::default()
        };

        assert!(profile.matches_application(Some("com.apple.Terminal"), None, None));
        assert!(!profile.matches_application(Some("com.apple.Safari"), None, None));
        assert!(!profile.matches_application(None, None, None));
    }

    #[test]
    fn test_detection_rules() {
        let mut profile = ApplicationProfile::default();
        profile.window_detection_rules.push(WindowDetectionRule {
            strategy: WindowDetectionStrategy::ProcessName,
            primary_identifier: "Terminal".to_string(),
            secondary_identifiers: Vec::new(),
            title_pattern: None,
            class_pattern: None,
        });

        assert!(profile.matches_application(None, Some("Terminal"), None));
        assert!(!profile.matches_application(None, Some("Safari"), None));
    }

    #[test]
    fn test_effective_positioning() {
        let mut profile = ApplicationProfile {
            default_positioning: PositioningRule::Auto,
            ..Default::default()
        };

        // Should return default positioning
        assert_eq!(
            profile.get_effective_positioning(None),
            PositioningRule::Auto
        );

        // Add custom rule
        profile.add_custom_rule(CustomHandlingRule {
            condition: "preferences".to_string(),
            action: CustomAction::ForcePositioning(PositioningRule::Floating),
            parameters: None,
            enabled: true,
        });

        // Should return custom positioning for matching window
        assert_eq!(
            profile.get_effective_positioning(Some("App preferences")),
            PositioningRule::Floating
        );

        // Should return default for non-matching window
        assert_eq!(
            profile.get_effective_positioning(Some("Main window")),
            PositioningRule::Auto
        );
    }

    #[test]
    fn test_compatibility_checking() {
        let full_profile = ApplicationProfile {
            compatibility_info: CompatibilityInfo {
                level: CompatibilityLevel::Full,
                issues: Vec::new(),
                workarounds: Vec::new(),
                version_notes: None,
            },
            ..Default::default()
        };

        let incompatible_profile = ApplicationProfile {
            compatibility_info: CompatibilityInfo {
                level: CompatibilityLevel::Incompatible,
                issues: Vec::new(),
                workarounds: Vec::new(),
                version_notes: None,
            },
            ..Default::default()
        };

        assert!(full_profile.is_tiling_compatible());
        assert!(!incompatible_profile.is_tiling_compatible());
    }

    #[test]
    fn test_preferred_patterns() {
        let mut profile = ApplicationProfile::default();
        let pattern_id = Uuid::new_v4();

        profile.add_preferred_pattern(pattern_id);
        assert!(profile.preferred_tiling_patterns.contains(&pattern_id));

        // Adding the same pattern shouldn't duplicate it
        profile.add_preferred_pattern(pattern_id);
        assert_eq!(profile.preferred_tiling_patterns.len(), 1);

        // Remove pattern
        assert!(profile.remove_preferred_pattern(&pattern_id));
        assert!(!profile.preferred_tiling_patterns.contains(&pattern_id));
    }

    #[test]
    fn test_profile_set() {
        let mut set = ApplicationProfileSet::new();

        let profile1 = ApplicationProfile {
            bundle_identifier: "com.apple.Terminal".to_string(),
            ..Default::default()
        };

        let profile2 = ApplicationProfile {
            bundle_identifier: "com.apple.Safari".to_string(),
            ..Default::default()
        };

        assert!(set.add_profile(profile1.clone()).is_ok());
        assert!(set.add_profile(profile2).is_ok());

        // Test duplicate prevention
        let duplicate_profile = ApplicationProfile {
            bundle_identifier: "com.apple.Terminal".to_string(),
            ..Default::default()
        };
        assert!(set.add_profile(duplicate_profile).is_err());

        // Test finding by bundle ID
        let found = set.find_by_bundle_id("com.apple.Terminal");
        assert!(found.is_some());
        assert_eq!(found.unwrap().bundle_identifier, "com.apple.Terminal");
    }

    #[test]
    fn test_default_profiles_creation() {
        let set = ApplicationProfileSet::create_with_defaults();
        assert!(set.is_ok());

        let set = set.unwrap();
        assert!(!set.profiles.is_empty());

        // Check that Terminal profile exists
        let terminal = set.find_by_bundle_id("com.apple.Terminal");
        assert!(terminal.is_some());
        assert!(!terminal.unwrap().is_user_profile); // Should be built-in
    }

    #[test]
    fn test_validation() {
        let mut profile = ApplicationProfile::default();
        profile.bundle_identifier = "com.example.app".to_string();
        profile.display_name = "Test App".to_string();
        assert!(profile.validate().is_ok());

        // Test empty bundle identifier
        profile.bundle_identifier = "".to_string();
        assert!(profile.validate().is_err());

        // Test invalid bundle identifier format
        profile.bundle_identifier = "invalid".to_string();
        assert!(profile.validate().is_err());

        // Test empty display name
        profile.bundle_identifier = "com.example.app".to_string();
        profile.display_name = "".to_string();
        assert!(profile.validate().is_err());
    }
}
