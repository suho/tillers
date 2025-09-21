use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Screen area definition with position and dimensions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScreenArea {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Preferred monitor orientation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrientationPreference {
    /// Use current orientation
    Current,
    /// Prefer landscape orientation
    Landscape,
    /// Prefer portrait orientation
    Portrait,
    /// Match primary monitor orientation
    MatchPrimary,
}

/// Monitor characteristics for identification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MonitorInfo {
    /// Display name from system
    pub display_name: String,
    /// Hardware model identifier
    pub model_identifier: Option<String>,
    /// Serial number if available
    pub serial_number: Option<String>,
    /// Physical size in millimeters
    pub physical_size_mm: Option<(u32, u32)>,
    /// Native resolution
    pub native_resolution: (u32, u32),
}

/// Defines workspace behavior for specific monitor setups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfiguration {
    /// Unique identifier
    pub id: Uuid,
    /// Reference to parent workspace
    pub workspace_id: Uuid,
    /// System monitor identification
    pub monitor_identifier: String,
    /// Additional monitor information for better matching
    pub monitor_info: Option<MonitorInfo>,
    /// Main tiling pattern for this monitor
    pub primary_pattern_id: Uuid,
    /// Fallback pattern for overflow windows
    pub secondary_pattern_id: Option<Uuid>,
    /// Screen area to use for tiling (excludes dock, menu bar, etc.)
    pub active_area: ScreenArea,
    /// Preferred monitor orientation
    pub orientation_preference: OrientationPreference,
    /// DPI scaling factor for window sizing
    pub scale_factor: f64,
    /// Whether this monitor is the primary display
    pub is_primary: bool,
    /// Whether to handle fullscreen windows specially on this monitor
    pub handle_fullscreen: bool,
}

impl MonitorConfiguration {
    /// Create a new monitor configuration with validation
    #[allow(clippy::too_many_arguments)] // Monitor definitions need explicit fields for serialization
    pub fn new(
        workspace_id: Uuid,
        monitor_identifier: String,
        monitor_info: Option<MonitorInfo>,
        primary_pattern_id: Uuid,
        secondary_pattern_id: Option<Uuid>,
        active_area: ScreenArea,
        orientation_preference: OrientationPreference,
        scale_factor: f64,
        is_primary: bool,
        handle_fullscreen: bool,
    ) -> Result<Self, MonitorConfigurationError> {
        let config = MonitorConfiguration {
            id: Uuid::new_v4(),
            workspace_id,
            monitor_identifier,
            monitor_info,
            primary_pattern_id,
            secondary_pattern_id,
            active_area,
            orientation_preference,
            scale_factor,
            is_primary,
            handle_fullscreen,
        };

        config.validate()?;
        Ok(config)
    }

    /// Check if this configuration matches a given monitor
    pub fn matches_monitor(&self, monitor_id: &str, monitor_info: Option<&MonitorInfo>) -> bool {
        // Primary match by identifier
        if self.monitor_identifier == monitor_id {
            return true;
        }

        // Enhanced matching using monitor info if available
        if let (Some(config_info), Some(current_info)) = (&self.monitor_info, monitor_info) {
            // Match by model identifier
            if let (Some(config_model), Some(current_model)) = (
                &config_info.model_identifier,
                &current_info.model_identifier,
            ) {
                if config_model == current_model {
                    return true;
                }
            }

            // Match by serial number (most reliable)
            if let (Some(config_serial), Some(current_serial)) =
                (&config_info.serial_number, &current_info.serial_number)
            {
                if config_serial == current_serial {
                    return true;
                }
            }

            // Match by display name and resolution
            if config_info.display_name == current_info.display_name
                && config_info.native_resolution == current_info.native_resolution
            {
                return true;
            }
        }

        false
    }

    /// Get the effective tiling pattern for the current window count
    pub fn get_effective_pattern_id(
        &self,
        window_count: usize,
        max_primary_windows: usize,
    ) -> Uuid {
        if window_count > max_primary_windows {
            self.secondary_pattern_id.unwrap_or(self.primary_pattern_id)
        } else {
            self.primary_pattern_id
        }
    }

    /// Calculate scaled dimensions for windows on this monitor
    pub fn scale_dimensions(&self, width: u32, height: u32) -> (u32, u32) {
        let scaled_width = (width as f64 * self.scale_factor).round() as u32;
        let scaled_height = (height as f64 * self.scale_factor).round() as u32;
        (scaled_width, scaled_height)
    }

    /// Calculate scaled position for windows on this monitor
    pub fn scale_position(&self, x: i32, y: i32) -> (i32, i32) {
        let scaled_x = (x as f64 * self.scale_factor).round() as i32;
        let scaled_y = (y as f64 * self.scale_factor).round() as i32;
        (scaled_x, scaled_y)
    }

    /// Get the usable area for window tiling (accounting for system UI)
    pub fn get_usable_area(&self, full_screen_area: &ScreenArea) -> ScreenArea {
        // Intersect the active area with the full screen area to ensure bounds
        let x = self.active_area.x.max(full_screen_area.x);
        let y = self.active_area.y.max(full_screen_area.y);

        let right = (self.active_area.x + self.active_area.width as i32)
            .min(full_screen_area.x + full_screen_area.width as i32);
        let bottom = (self.active_area.y + self.active_area.height as i32)
            .min(full_screen_area.y + full_screen_area.height as i32);

        let width = (right - x).max(0) as u32;
        let height = (bottom - y).max(0) as u32;

        ScreenArea {
            x,
            y,
            width,
            height,
        }
    }

    /// Check if the monitor orientation matches the preference
    pub fn orientation_matches(&self, monitor_width: u32, monitor_height: u32) -> bool {
        match self.orientation_preference {
            OrientationPreference::Current => true,
            OrientationPreference::Landscape => monitor_width >= monitor_height,
            OrientationPreference::Portrait => monitor_height > monitor_width,
            OrientationPreference::MatchPrimary => {
                // This would need to be checked against the primary monitor
                // For now, assume it matches
                true
            }
        }
    }

    /// Update the active area for this monitor configuration
    pub fn update_active_area(
        &mut self,
        new_area: ScreenArea,
    ) -> Result<(), MonitorConfigurationError> {
        if new_area.width == 0 || new_area.height == 0 {
            return Err(MonitorConfigurationError::InvalidActiveArea);
        }

        self.active_area = new_area;
        Ok(())
    }

    /// Validate the monitor configuration
    pub fn validate(&self) -> Result<(), MonitorConfigurationError> {
        // Validate monitor identifier
        if self.monitor_identifier.trim().is_empty() {
            return Err(MonitorConfigurationError::EmptyMonitorIdentifier);
        }

        // Validate active area
        if self.active_area.width == 0 || self.active_area.height == 0 {
            return Err(MonitorConfigurationError::InvalidActiveArea);
        }

        // Validate scale factor
        if self.scale_factor <= 0.0 || self.scale_factor > 5.0 {
            return Err(MonitorConfigurationError::InvalidScaleFactor(
                self.scale_factor,
            ));
        }

        // Validate monitor info if present
        if let Some(ref info) = self.monitor_info {
            if info.display_name.trim().is_empty() {
                return Err(MonitorConfigurationError::EmptyDisplayName);
            }

            if info.native_resolution.0 == 0 || info.native_resolution.1 == 0 {
                return Err(MonitorConfigurationError::InvalidNativeResolution);
            }
        }

        Ok(())
    }
}

/// Collection of monitor configurations for a workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfigurationSet {
    pub workspace_id: Uuid,
    pub configurations: Vec<MonitorConfiguration>,
    pub fallback_configuration: Option<MonitorConfiguration>,
}

impl MonitorConfigurationSet {
    /// Create a new monitor configuration set
    pub fn new(workspace_id: Uuid) -> Self {
        MonitorConfigurationSet {
            workspace_id,
            configurations: Vec::new(),
            fallback_configuration: None,
        }
    }

    /// Add a monitor configuration to the set
    pub fn add_configuration(
        &mut self,
        config: MonitorConfiguration,
    ) -> Result<(), MonitorConfigurationError> {
        if config.workspace_id != self.workspace_id {
            return Err(MonitorConfigurationError::WorkspaceMismatch);
        }

        // Check for duplicate monitor identifiers
        if self
            .configurations
            .iter()
            .any(|c| c.monitor_identifier == config.monitor_identifier)
        {
            return Err(MonitorConfigurationError::DuplicateMonitorIdentifier(
                config.monitor_identifier,
            ));
        }

        self.configurations.push(config);
        Ok(())
    }

    /// Find a configuration for a specific monitor
    pub fn find_configuration(
        &self,
        monitor_id: &str,
        monitor_info: Option<&MonitorInfo>,
    ) -> Option<&MonitorConfiguration> {
        self.configurations
            .iter()
            .find(|config| config.matches_monitor(monitor_id, monitor_info))
            .or(self.fallback_configuration.as_ref())
    }

    /// Get the primary monitor configuration
    pub fn get_primary_configuration(&self) -> Option<&MonitorConfiguration> {
        self.configurations.iter().find(|config| config.is_primary)
    }

    /// Update configuration for a specific monitor
    pub fn update_configuration<F>(
        &mut self,
        monitor_id: &str,
        updater: F,
    ) -> Result<(), MonitorConfigurationError>
    where
        F: FnOnce(&mut MonitorConfiguration) -> Result<(), MonitorConfigurationError>,
    {
        if let Some(config) = self
            .configurations
            .iter_mut()
            .find(|c| c.monitor_identifier == monitor_id)
        {
            updater(config)
        } else {
            Err(MonitorConfigurationError::MonitorNotFound(
                monitor_id.to_string(),
            ))
        }
    }
}

/// Errors that can occur with monitor configurations
#[derive(Debug, thiserror::Error)]
pub enum MonitorConfigurationError {
    #[error("Monitor identifier cannot be empty")]
    EmptyMonitorIdentifier,

    #[error("Display name cannot be empty")]
    EmptyDisplayName,

    #[error("Active area must have non-zero dimensions")]
    InvalidActiveArea,

    #[error("Scale factor must be between 0.0 and 5.0, got {0}")]
    InvalidScaleFactor(f64),

    #[error("Native resolution must have non-zero dimensions")]
    InvalidNativeResolution,

    #[error("Workspace ID mismatch")]
    WorkspaceMismatch,

    #[error("Duplicate monitor identifier: {0}")]
    DuplicateMonitorIdentifier(String),

    #[error("Monitor not found: {0}")]
    MonitorNotFound(String),

    #[error("Monitor configuration is outside screen bounds")]
    ConfigurationOutOfBounds,
}

impl Default for MonitorConfiguration {
    fn default() -> Self {
        MonitorConfiguration {
            id: Uuid::new_v4(),
            workspace_id: Uuid::new_v4(),
            monitor_identifier: "main".to_string(),
            monitor_info: None,
            primary_pattern_id: Uuid::new_v4(),
            secondary_pattern_id: None,
            active_area: ScreenArea {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            orientation_preference: OrientationPreference::Current,
            scale_factor: 1.0,
            is_primary: true,
            handle_fullscreen: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_monitor_configuration_valid() {
        let workspace_id = Uuid::new_v4();
        let pattern_id = Uuid::new_v4();
        let active_area = ScreenArea {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };

        let config = MonitorConfiguration::new(
            workspace_id,
            "main".to_string(),
            None,
            pattern_id,
            None,
            active_area,
            OrientationPreference::Current,
            1.0,
            true,
            true,
        );

        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.workspace_id, workspace_id);
        assert_eq!(config.monitor_identifier, "main");
        assert_eq!(config.primary_pattern_id, pattern_id);
    }

    #[test]
    fn test_new_monitor_configuration_invalid_scale() {
        let workspace_id = Uuid::new_v4();
        let pattern_id = Uuid::new_v4();
        let active_area = ScreenArea {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };

        let config = MonitorConfiguration::new(
            workspace_id,
            "main".to_string(),
            None,
            pattern_id,
            None,
            active_area,
            OrientationPreference::Current,
            0.0, // Invalid scale factor
            true,
            true,
        );

        assert!(config.is_err());
    }

    #[test]
    fn test_monitor_matching() {
        let config = MonitorConfiguration {
            monitor_identifier: "display1".to_string(),
            monitor_info: Some(MonitorInfo {
                display_name: "LG Monitor".to_string(),
                model_identifier: Some("LG27UP850".to_string()),
                serial_number: Some("123456789".to_string()),
                physical_size_mm: Some((597, 336)),
                native_resolution: (3840, 2160),
            }),
            ..Default::default()
        };

        // Test exact identifier match
        assert!(config.matches_monitor("display1", None));

        // Test model identifier match
        let current_info = MonitorInfo {
            display_name: "LG Monitor".to_string(),
            model_identifier: Some("LG27UP850".to_string()),
            serial_number: None,
            physical_size_mm: None,
            native_resolution: (3840, 2160),
        };
        assert!(config.matches_monitor("different_id", Some(&current_info)));

        // Test serial number match
        let current_info_serial = MonitorInfo {
            display_name: "Different Name".to_string(),
            model_identifier: Some("DifferentModel".to_string()),
            serial_number: Some("123456789".to_string()),
            physical_size_mm: None,
            native_resolution: (1920, 1080),
        };
        assert!(config.matches_monitor("different_id", Some(&current_info_serial)));

        // Test no match
        let no_match_info = MonitorInfo {
            display_name: "Dell Monitor".to_string(),
            model_identifier: Some("Dell2415B".to_string()),
            serial_number: Some("987654321".to_string()),
            physical_size_mm: None,
            native_resolution: (1920, 1200),
        };
        assert!(!config.matches_monitor("different_id", Some(&no_match_info)));
    }

    #[test]
    fn test_effective_pattern_selection() {
        let primary_pattern = Uuid::new_v4();
        let secondary_pattern = Uuid::new_v4();

        let config = MonitorConfiguration {
            primary_pattern_id: primary_pattern,
            secondary_pattern_id: Some(secondary_pattern),
            ..Default::default()
        };

        // Should use primary pattern for small window counts
        assert_eq!(config.get_effective_pattern_id(3, 5), primary_pattern);

        // Should use secondary pattern for large window counts
        assert_eq!(config.get_effective_pattern_id(8, 5), secondary_pattern);

        // Test with no secondary pattern
        let config_no_secondary = MonitorConfiguration {
            primary_pattern_id: primary_pattern,
            secondary_pattern_id: None,
            ..Default::default()
        };
        assert_eq!(
            config_no_secondary.get_effective_pattern_id(8, 5),
            primary_pattern
        );
    }

    #[test]
    fn test_scaling() {
        let config = MonitorConfiguration {
            scale_factor: 2.0,
            ..Default::default()
        };

        assert_eq!(config.scale_dimensions(100, 200), (200, 400));
        assert_eq!(config.scale_position(50, 75), (100, 150));
    }

    #[test]
    fn test_usable_area_calculation() {
        let config = MonitorConfiguration {
            active_area: ScreenArea {
                x: 10,
                y: 30,
                width: 1900,
                height: 1000,
            },
            ..Default::default()
        };

        let full_screen = ScreenArea {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let usable = config.get_usable_area(&full_screen);

        assert_eq!(usable.x, 10);
        assert_eq!(usable.y, 30);
        assert_eq!(usable.width, 1900);
        assert_eq!(usable.height, 1000);
    }

    #[test]
    fn test_monitor_configuration_set() {
        let workspace_id = Uuid::new_v4();
        let mut config_set = MonitorConfigurationSet::new(workspace_id);

        let config1 = MonitorConfiguration {
            workspace_id,
            monitor_identifier: "display1".to_string(),
            is_primary: true,
            ..Default::default()
        };

        let config2 = MonitorConfiguration {
            workspace_id,
            monitor_identifier: "display2".to_string(),
            is_primary: false,
            ..Default::default()
        };

        assert!(config_set.add_configuration(config1.clone()).is_ok());
        assert!(config_set.add_configuration(config2).is_ok());

        // Test finding configuration
        assert!(config_set.find_configuration("display1", None).is_some());
        assert!(config_set.find_configuration("nonexistent", None).is_none());

        // Test primary configuration
        let primary = config_set.get_primary_configuration();
        assert!(primary.is_some());
        assert_eq!(primary.unwrap().monitor_identifier, "display1");

        // Test duplicate identifier
        let duplicate_config = MonitorConfiguration {
            workspace_id,
            monitor_identifier: "display1".to_string(),
            ..Default::default()
        };
        assert!(config_set.add_configuration(duplicate_config).is_err());
    }

    #[test]
    fn test_orientation_matching() {
        let config = MonitorConfiguration {
            orientation_preference: OrientationPreference::Landscape,
            ..Default::default()
        };

        assert!(config.orientation_matches(1920, 1080)); // Landscape
        assert!(!config.orientation_matches(1080, 1920)); // Portrait

        let portrait_config = MonitorConfiguration {
            orientation_preference: OrientationPreference::Portrait,
            ..Default::default()
        };

        assert!(!portrait_config.orientation_matches(1920, 1080)); // Landscape
        assert!(portrait_config.orientation_matches(1080, 1920)); // Portrait
    }

    #[test]
    fn test_validation() {
        let mut config = MonitorConfiguration::default();
        assert!(config.validate().is_ok());

        // Test empty identifier
        config.monitor_identifier = "".to_string();
        assert!(config.validate().is_err());

        // Test invalid scale factor
        config.monitor_identifier = "display1".to_string();
        config.scale_factor = 0.0;
        assert!(config.validate().is_err());

        // Test invalid active area
        config.scale_factor = 1.0;
        config.active_area.width = 0;
        assert!(config.validate().is_err());
    }
}
