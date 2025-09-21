use crate::models::{
    workspace::Workspace,
    tiling_pattern::TilingPattern,
    window_rule::WindowRule,
    monitor_configuration::MonitorConfiguration,
    keyboard_mapping::KeyboardMapping,
    application_profile::ApplicationProfile,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum ConfigParseError {
    #[error("File IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("TOML parsing error: {0}")]
    TomlError(#[from] toml::de::Error),
    #[error("Validation error: {message}")]
    ValidationError { message: String },
    #[error("Migration error: {message}")]
    MigrationError { message: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub workspaces: Vec<Workspace>,
    pub patterns: Vec<TilingPattern>,
    pub window_rules: Vec<WindowRule>,
    pub monitor_configs: Vec<MonitorConfiguration>,
    pub keyboard_mappings: Vec<KeyboardMapping>,
    pub application_profiles: Vec<ApplicationProfile>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigFile {
    pub version: String,
    pub config: WorkspaceConfig,
}

#[derive(Debug, Serialize, Deserialize)]
struct LegacyKeyboardMapping {
    pub id: Uuid,
    pub shortcut_combination: String,
    pub action_type: String,
    pub target_id: Option<Uuid>,
    pub parameters: Option<serde_json::Value>,
    pub enabled: bool,
    pub global_scope: bool,
    pub modifier_preference: Option<String>,
}

pub struct ConfigParser {
    migration_warnings: Vec<String>,
}

impl ConfigParser {
    pub fn new() -> Self {
        Self {
            migration_warnings: Vec::new(),
        }
    }

    pub fn parse_workspace_file<P: AsRef<Path>>(&mut self, path: P) -> Result<WorkspaceConfig, ConfigParseError> {
        let content = std::fs::read_to_string(path)?;
        let config_file: ConfigFile = toml::from_str(&content)?;
        
        self.validate_config(&config_file.config)?;
        self.migrate_keyboard_shortcuts(config_file.config)
    }

    pub fn parse_workspaces_toml(&mut self, content: &str) -> Result<Vec<Workspace>, ConfigParseError> {
        let workspaces: Vec<Workspace> = toml::from_str(content)?;
        self.validate_workspaces(&workspaces)?;
        Ok(workspaces)
    }

    pub fn parse_patterns_toml(&mut self, content: &str) -> Result<Vec<TilingPattern>, ConfigParseError> {
        let patterns: Vec<TilingPattern> = toml::from_str(content)?;
        self.validate_patterns(&patterns)?;
        Ok(patterns)
    }

    pub fn parse_keybindings_toml(&mut self, content: &str) -> Result<Vec<KeyboardMapping>, ConfigParseError> {
        let raw_content: toml::Value = toml::from_str(content)?;
        
        if let Some(legacy_mappings) = self.detect_legacy_keyboard_format(&raw_content) {
            self.migration_warnings.push(
                "Detected Command-based keyboard shortcuts. Migrating to Option-based shortcuts.".to_string()
            );
            return self.migrate_legacy_keyboard_mappings(legacy_mappings);
        }

        let mappings: Vec<KeyboardMapping> = toml::from_str(content)?;
        self.validate_keyboard_mappings(&mappings)?;
        Ok(mappings)
    }

    pub fn parse_applications_toml(&mut self, content: &str) -> Result<Vec<ApplicationProfile>, ConfigParseError> {
        let profiles: Vec<ApplicationProfile> = toml::from_str(content)?;
        self.validate_application_profiles(&profiles)?;
        Ok(profiles)
    }

    pub fn parse_monitors_toml(&mut self, content: &str) -> Result<Vec<MonitorConfiguration>, ConfigParseError> {
        let configs: Vec<MonitorConfiguration> = toml::from_str(content)?;
        self.validate_monitor_configurations(&configs)?;
        Ok(configs)
    }

    pub fn get_migration_warnings(&self) -> &[String] {
        &self.migration_warnings
    }

    fn validate_config(&self, config: &WorkspaceConfig) -> Result<(), ConfigParseError> {
        self.validate_workspaces(&config.workspaces)?;
        self.validate_patterns(&config.patterns)?;
        self.validate_keyboard_mappings(&config.keyboard_mappings)?;
        self.validate_application_profiles(&config.application_profiles)?;
        self.validate_monitor_configurations(&config.monitor_configs)?;
        self.validate_cross_references(config)?;
        Ok(())
    }

    fn validate_workspaces(&self, workspaces: &[Workspace]) -> Result<(), ConfigParseError> {
        let mut names = std::collections::HashSet::new();
        let mut shortcuts = std::collections::HashSet::new();

        for workspace in workspaces {
            if !names.insert(&workspace.name) {
                return Err(ConfigParseError::ValidationError {
                    message: format!("Duplicate workspace name: {}", workspace.name),
                });
            }

            if !shortcuts.insert(&workspace.keyboard_shortcut) {
                return Err(ConfigParseError::ValidationError {
                    message: format!("Duplicate keyboard shortcut: {}", workspace.keyboard_shortcut),
                });
            }

            if workspace.name.is_empty() {
                return Err(ConfigParseError::ValidationError {
                    message: "Workspace name cannot be empty".to_string(),
                });
            }

            if workspace.name.len() > 100 {
                return Err(ConfigParseError::ValidationError {
                    message: format!("Workspace name too long: {} (max 100 chars)", workspace.name),
                });
            }
        }

        Ok(())
    }

    fn validate_patterns(&self, patterns: &[TilingPattern]) -> Result<(), ConfigParseError> {
        for pattern in patterns {
            if pattern.main_area_ratio < 0.1 || pattern.main_area_ratio > 0.9 {
                return Err(ConfigParseError::ValidationError {
                    message: format!(
                        "Main area ratio {} must be between 0.1 and 0.9",
                        pattern.main_area_ratio
                    ),
                });
            }

            if pattern.max_windows == 0 {
                return Err(ConfigParseError::ValidationError {
                    message: "Max windows must be positive".to_string(),
                });
            }
        }

        Ok(())
    }

    fn validate_keyboard_mappings(&self, mappings: &[KeyboardMapping]) -> Result<(), ConfigParseError> {
        let mut combinations = std::collections::HashSet::new();

        for mapping in mappings {
            if !combinations.insert(&mapping.shortcut_combination) {
                return Err(ConfigParseError::ValidationError {
                    message: format!("Duplicate shortcut combination: {}", mapping.shortcut_combination),
                });
            }

            self.validate_shortcut_format(&mapping.shortcut_combination)?;
            
            if mapping.shortcut_combination.contains("cmd+") {
                self.migration_warnings.push(format!(
                    "Command-based shortcut detected: {}. Consider migrating to Option key.",
                    mapping.shortcut_combination
                ));
            }
        }

        Ok(())
    }

    fn validate_application_profiles(&self, profiles: &[ApplicationProfile]) -> Result<(), ConfigParseError> {
        let mut bundle_ids = std::collections::HashSet::new();

        for profile in profiles {
            if !bundle_ids.insert(&profile.bundle_identifier) {
                return Err(ConfigParseError::ValidationError {
                    message: format!("Duplicate bundle identifier: {}", profile.bundle_identifier),
                });
            }
        }

        Ok(())
    }

    fn validate_monitor_configurations(&self, configs: &[MonitorConfiguration]) -> Result<(), ConfigParseError> {
        for config in configs {
            if config.scale_factor <= 0.0 {
                return Err(ConfigParseError::ValidationError {
                    message: "Scale factor must be positive".to_string(),
                });
            }
        }

        Ok(())
    }

    fn validate_cross_references(&self, config: &WorkspaceConfig) -> Result<(), ConfigParseError> {
        let pattern_ids: std::collections::HashSet<_> = 
            config.patterns.iter().map(|p| &p.id).collect();
        let workspace_ids: std::collections::HashSet<_> = 
            config.workspaces.iter().map(|w| &w.id).collect();

        for workspace in &config.workspaces {
            if !pattern_ids.contains(&workspace.tiling_pattern_id) {
                return Err(ConfigParseError::ValidationError {
                    message: format!(
                        "Workspace '{}' references non-existent tiling pattern: {}",
                        workspace.name,
                        workspace.tiling_pattern_id
                    ),
                });
            }
        }

        for mapping in &config.keyboard_mappings {
            if let Some(target_id) = &mapping.target_id {
                if !workspace_ids.contains(target_id) {
                    return Err(ConfigParseError::ValidationError {
                        message: format!(
                            "Keyboard mapping references non-existent workspace: {}",
                            target_id
                        ),
                    });
                }
            }
        }

        Ok(())
    }

    fn validate_shortcut_format(&self, combination: &str) -> Result<(), ConfigParseError> {
        let regex = regex::Regex::new(r"^(cmd|ctrl|opt|shift)(\+(cmd|ctrl|opt|shift))*\+[a-zA-Z0-9F1-F12]+$")
            .map_err(|e| ConfigParseError::ValidationError {
                message: format!("Invalid regex: {}", e),
            })?;

        if !regex.is_match(combination) {
            return Err(ConfigParseError::ValidationError {
                message: format!("Invalid shortcut format: {}", combination),
            });
        }

        Ok(())
    }

    fn detect_legacy_keyboard_format(&self, raw_content: &toml::Value) -> Option<Vec<LegacyKeyboardMapping>> {
        if let Some(array) = raw_content.as_array() {
            for item in array {
                if let Some(table) = item.as_table() {
                    if table.contains_key("modifier_preference") || 
                       (table.contains_key("shortcut_combination") && 
                        table.get("shortcut_combination")
                            .and_then(|v| v.as_str())
                            .map(|s| s.contains("cmd+"))
                            .unwrap_or(false)) {
                        
                        if let Ok(legacy_mappings) = toml::from_str::<Vec<LegacyKeyboardMapping>>(
                            &toml::to_string(raw_content).unwrap_or_default()
                        ) {
                            return Some(legacy_mappings);
                        }
                    }
                }
            }
        }
        None
    }

    fn migrate_legacy_keyboard_mappings(&mut self, legacy_mappings: Vec<LegacyKeyboardMapping>) -> Result<Vec<KeyboardMapping>, ConfigParseError> {
        let mut migrated_mappings = Vec::new();

        for legacy in legacy_mappings {
            let migrated_combination = self.migrate_shortcut_combination(&legacy.shortcut_combination)?;
            
            if migrated_combination != legacy.shortcut_combination {
                self.migration_warnings.push(format!(
                    "Migrated shortcut: {} → {}",
                    legacy.shortcut_combination,
                    migrated_combination
                ));
            }

            migrated_mappings.push(KeyboardMapping {
                id: legacy.id,
                shortcut_combination: migrated_combination,
                action_type: legacy.action_type,
                target_id: legacy.target_id,
                parameters: legacy.parameters,
                enabled: legacy.enabled,
                global_scope: legacy.global_scope,
            });
        }

        Ok(migrated_mappings)
    }

    fn migrate_shortcut_combination(&self, combination: &str) -> Result<String, ConfigParseError> {
        if combination.starts_with("cmd+") {
            Ok(combination.replace("cmd+", "opt+"))
        } else if combination.contains("+cmd+") {
            Ok(combination.replace("+cmd+", "+opt+"))
        } else {
            Ok(combination.to_string())
        }
    }

    fn migrate_keyboard_shortcuts(&mut self, mut config: WorkspaceConfig) -> Result<WorkspaceConfig, ConfigParseError> {
        let mut migration_needed = false;

        for workspace in &mut config.workspaces {
            if workspace.keyboard_shortcut.contains("cmd+") {
                let old_shortcut = workspace.keyboard_shortcut.clone();
                workspace.keyboard_shortcut = self.migrate_shortcut_combination(&workspace.keyboard_shortcut)?;
                
                if workspace.keyboard_shortcut != old_shortcut {
                    migration_needed = true;
                    self.migration_warnings.push(format!(
                        "Migrated workspace '{}' shortcut: {} → {}",
                        workspace.name,
                        old_shortcut,
                        workspace.keyboard_shortcut
                    ));
                }
            }
        }

        if migration_needed {
            self.migration_warnings.push(
                "Configuration migration completed. Command-based shortcuts converted to Option-based.".to_string()
            );
        }

        Ok(config)
    }
}

impl Default for ConfigParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::workspace::Workspace;
    use crate::models::tiling_pattern::{TilingPattern, LayoutAlgorithm, ResizeBehavior};
    use uuid::Uuid;
    use chrono::Utc;

    #[test]
    fn test_parse_valid_workspace_toml() {
        let mut parser = ConfigParser::new();
        
        let toml_content = r#"
[[workspace]]
id = "550e8400-e29b-41d4-a716-446655440000"
name = "Development"
description = "Main development workspace"
keyboard_shortcut = "opt+1"
tiling_pattern_id = "550e8400-e29b-41d4-a716-446655440001"
auto_arrange = true
created_at = "2024-01-01T00:00:00Z"
last_used = "2024-01-01T00:00:00Z"
"#;

        let result = parser.parse_workspaces_toml(toml_content);
        assert!(result.is_ok());
        
        let workspaces = result.unwrap();
        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces[0].name, "Development");
        assert_eq!(workspaces[0].keyboard_shortcut, "opt+1");
    }

    #[test]
    fn test_duplicate_workspace_names() {
        let mut parser = ConfigParser::new();
        
        let toml_content = r#"
[[workspace]]
id = "550e8400-e29b-41d4-a716-446655440000"
name = "Development"
keyboard_shortcut = "opt+1"
tiling_pattern_id = "550e8400-e29b-41d4-a716-446655440001"
auto_arrange = true
created_at = "2024-01-01T00:00:00Z"
last_used = "2024-01-01T00:00:00Z"

[[workspace]]
id = "550e8400-e29b-41d4-a716-446655440002"
name = "Development"
keyboard_shortcut = "opt+2"
tiling_pattern_id = "550e8400-e29b-41d4-a716-446655440001"
auto_arrange = true
created_at = "2024-01-01T00:00:00Z"
last_used = "2024-01-01T00:00:00Z"
"#;

        let result = parser.parse_workspaces_toml(toml_content);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Duplicate workspace name"));
    }

    #[test]
    fn test_migration_from_command_to_option() {
        let mut parser = ConfigParser::new();
        
        let legacy_mapping = LegacyKeyboardMapping {
            id: Uuid::new_v4(),
            shortcut_combination: "cmd+1".to_string(),
            action_type: "SWITCH_WORKSPACE".to_string(),
            target_id: Some(Uuid::new_v4()),
            parameters: None,
            enabled: true,
            global_scope: true,
            modifier_preference: Some("cmd".to_string()),
        };

        let result = parser.migrate_legacy_keyboard_mappings(vec![legacy_mapping]);
        assert!(result.is_ok());
        
        let migrated = result.unwrap();
        assert_eq!(migrated[0].shortcut_combination, "opt+1");
        assert!(!parser.get_migration_warnings().is_empty());
    }

    #[test]
    fn test_shortcut_format_validation() {
        let parser = ConfigParser::new();
        
        assert!(parser.validate_shortcut_format("opt+1").is_ok());
        assert!(parser.validate_shortcut_format("opt+shift+1").is_ok());
        assert!(parser.validate_shortcut_format("cmd+ctrl+alt+1").is_err());
        assert!(parser.validate_shortcut_format("invalid").is_err());
    }
}