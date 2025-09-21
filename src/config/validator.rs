use crate::config::parser::{ConfigParseError, WorkspaceConfig};
use crate::models::{
    application_profile::ApplicationProfile,
    keyboard_mapping::{ActionParameters, KeyboardMapping, ModifierKey, ShortcutCombination},
    monitor_configuration::MonitorConfiguration,
    tiling_pattern::TilingPattern,
    window_rule::WindowRule,
    workspace::Workspace,
};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ValidationRule {
    pub name: String,
    pub description: String,
    pub severity: ValidationSeverity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub rule: ValidationRule,
    pub message: String,
    pub entity_id: Option<Uuid>,
    pub entity_type: String,
}

pub struct ConfigValidator {
    rules: Vec<ValidationRule>,
    keyboard_shortcut_regex: Regex,
    bundle_id_regex: Regex,
}

impl ConfigValidator {
    pub fn new() -> Result<Self, ConfigParseError> {
        let keyboard_shortcut_regex =
            Regex::new(r"^(cmd|ctrl|opt|shift)(\+(cmd|ctrl|opt|shift))*\+[a-zA-Z0-9F1-F12]+$")
                .map_err(|e| ConfigParseError::Validation {
                    message: format!("Failed to compile keyboard shortcut regex: {}", e),
                })?;

        let bundle_id_regex = Regex::new(r"^[a-zA-Z0-9]+([\.-][a-zA-Z0-9]+)*$").map_err(|e| {
            ConfigParseError::Validation {
                message: format!("Failed to compile bundle ID regex: {}", e),
            }
        })?;

        Ok(Self {
            rules: Self::default_rules(),
            keyboard_shortcut_regex,
            bundle_id_regex,
        })
    }

    pub fn validate_full_config(&self, config: &WorkspaceConfig) -> Vec<ValidationResult> {
        let mut results = Vec::new();

        results.extend(self.validate_workspaces(&config.workspaces));
        results.extend(self.validate_tiling_patterns(&config.patterns));
        results.extend(self.validate_window_rules(&config.window_rules));
        results.extend(self.validate_keyboard_mappings(&config.keyboard_mappings));
        results.extend(self.validate_application_profiles(&config.application_profiles));
        results.extend(self.validate_monitor_configurations(&config.monitor_configs));
        results.extend(self.validate_cross_references(config));
        results.extend(self.validate_performance_implications(config));

        results
    }

    pub fn validate_workspaces(&self, workspaces: &[Workspace]) -> Vec<ValidationResult> {
        let mut results = Vec::new();
        let mut name_map = HashMap::new();
        let mut shortcut_map = HashMap::new();

        for workspace in workspaces {
            if let Some(existing_id) = name_map.insert(&workspace.name, workspace.id) {
                if existing_id != workspace.id {
                    results.push(ValidationResult {
                        rule: self.get_rule("duplicate_workspace_name").unwrap(),
                        message: format!(
                            "Workspace name '{}' is used by multiple workspaces",
                            workspace.name
                        ),
                        entity_id: Some(workspace.id),
                        entity_type: "Workspace".to_string(),
                    });
                }
            }

            if let Some(existing_id) =
                shortcut_map.insert(&workspace.keyboard_shortcut, workspace.id)
            {
                if existing_id != workspace.id {
                    results.push(ValidationResult {
                        rule: self.get_rule("duplicate_keyboard_shortcut").unwrap(),
                        message: format!(
                            "Keyboard shortcut '{}' is used by multiple workspaces",
                            workspace.keyboard_shortcut
                        ),
                        entity_id: Some(workspace.id),
                        entity_type: "Workspace".to_string(),
                    });
                }
            }

            if workspace.name.is_empty() {
                results.push(ValidationResult {
                    rule: self.get_rule("empty_workspace_name").unwrap(),
                    message: "Workspace name cannot be empty".to_string(),
                    entity_id: Some(workspace.id),
                    entity_type: "Workspace".to_string(),
                });
            }

            if workspace.name.len() > 100 {
                results.push(ValidationResult {
                    rule: self.get_rule("workspace_name_too_long").unwrap(),
                    message: format!("Workspace name '{}' exceeds 100 characters", workspace.name),
                    entity_id: Some(workspace.id),
                    entity_type: "Workspace".to_string(),
                });
            }

            if !self
                .keyboard_shortcut_regex
                .is_match(&workspace.keyboard_shortcut)
            {
                results.push(ValidationResult {
                    rule: self.get_rule("invalid_keyboard_shortcut").unwrap(),
                    message: format!(
                        "Invalid keyboard shortcut format: '{}'",
                        workspace.keyboard_shortcut
                    ),
                    entity_id: Some(workspace.id),
                    entity_type: "Workspace".to_string(),
                });
            }

            if workspace.keyboard_shortcut.contains("cmd+") {
                results.push(ValidationResult {
                    rule: self.get_rule("legacy_command_shortcut").unwrap(),
                    message: format!("Consider migrating Command-based shortcut '{}' to Option key for better compatibility", workspace.keyboard_shortcut),
                    entity_id: Some(workspace.id),
                    entity_type: "Workspace".to_string(),
                });
            }

            if workspace.description.as_ref().map(|d| d.len()).unwrap_or(0) > 500 {
                results.push(ValidationResult {
                    rule: self.get_rule("workspace_description_too_long").unwrap(),
                    message: "Workspace description exceeds 500 characters".to_string(),
                    entity_id: Some(workspace.id),
                    entity_type: "Workspace".to_string(),
                });
            }
        }

        results
    }

    pub fn validate_tiling_patterns(&self, patterns: &[TilingPattern]) -> Vec<ValidationResult> {
        let mut results = Vec::new();
        let mut name_map = HashMap::new();

        for pattern in patterns {
            if let Some(existing_id) = name_map.insert(&pattern.name, pattern.id) {
                if existing_id != pattern.id {
                    results.push(ValidationResult {
                        rule: self.get_rule("duplicate_pattern_name").unwrap(),
                        message: format!(
                            "Tiling pattern name '{}' is used by multiple patterns",
                            pattern.name
                        ),
                        entity_id: Some(pattern.id),
                        entity_type: "TilingPattern".to_string(),
                    });
                }
            }

            if pattern.main_area_ratio < 0.1 || pattern.main_area_ratio > 0.9 {
                results.push(ValidationResult {
                    rule: self.get_rule("invalid_main_area_ratio").unwrap(),
                    message: format!(
                        "Main area ratio {} must be between 0.1 and 0.9",
                        pattern.main_area_ratio
                    ),
                    entity_id: Some(pattern.id),
                    entity_type: "TilingPattern".to_string(),
                });
            }

            if pattern.max_windows == 0 {
                results.push(ValidationResult {
                    rule: self.get_rule("zero_max_windows").unwrap(),
                    message: "Maximum windows must be greater than zero".to_string(),
                    entity_id: Some(pattern.id),
                    entity_type: "TilingPattern".to_string(),
                });
            }

            if pattern.max_windows > 50 {
                results.push(ValidationResult {
                    rule: self.get_rule("high_max_windows").unwrap(),
                    message: format!(
                        "High maximum windows count ({}) may impact performance",
                        pattern.max_windows
                    ),
                    entity_id: Some(pattern.id),
                    entity_type: "TilingPattern".to_string(),
                });
            }
        }

        results
    }

    pub fn validate_window_rules(&self, rules: &[WindowRule]) -> Vec<ValidationResult> {
        let mut results = Vec::new();

        for rule in rules {
            if let Some(ref pattern) = rule.window_title_pattern {
                if let Err(e) = Regex::new(pattern) {
                    results.push(ValidationResult {
                        rule: self.get_rule("invalid_window_title_regex").unwrap(),
                        message: format!("Invalid window title regex '{}': {}", pattern, e),
                        entity_id: Some(rule.id),
                        entity_type: "WindowRule".to_string(),
                    });
                }
            }

            if let Some(ref geometry) = rule.fixed_geometry {
                if geometry.width == 0 || geometry.height == 0 {
                    results.push(ValidationResult {
                        rule: self.get_rule("invalid_fixed_size").unwrap(),
                        message: "Fixed geometry dimensions must be positive".to_string(),
                        entity_id: Some(rule.id),
                        entity_type: "WindowRule".to_string(),
                    });
                }
            }
        }

        results
    }

    pub fn validate_keyboard_mappings(
        &self,
        mappings: &[KeyboardMapping],
    ) -> Vec<ValidationResult> {
        let mut results = Vec::new();
        let mut shortcut_map: HashMap<String, Uuid> = HashMap::new();

        for mapping in mappings {
            let signature = mapping.shortcut_combination.to_config_string();

            if let Some(existing_id) = shortcut_map.insert(signature.clone(), mapping.id) {
                if existing_id != mapping.id {
                    results.push(ValidationResult {
                        rule: self.get_rule("duplicate_keyboard_shortcut").unwrap(),
                        message: format!(
                            "Keyboard shortcut '{}' is used by multiple mappings",
                            signature
                        ),
                        entity_id: Some(mapping.id),
                        entity_type: "KeyboardMapping".to_string(),
                    });
                }
            }

            if let Err(err) = mapping.shortcut_combination.validate() {
                results.push(ValidationResult {
                    rule: self.get_rule("invalid_keyboard_shortcut").unwrap(),
                    message: format!("Invalid keyboard shortcut '{}': {}", signature, err),
                    entity_id: Some(mapping.id),
                    entity_type: "KeyboardMapping".to_string(),
                });
            } else if !self.keyboard_shortcut_regex.is_match(&signature) {
                results.push(ValidationResult {
                    rule: self.get_rule("invalid_keyboard_shortcut").unwrap(),
                    message: format!("Invalid keyboard shortcut format: '{}'", signature),
                    entity_id: Some(mapping.id),
                    entity_type: "KeyboardMapping".to_string(),
                });
            }

            if mapping
                .shortcut_combination
                .contains_modifier(&ModifierKey::Command)
                && !mapping.shortcut_combination.has_option_modifier()
            {
                results.push(ValidationResult {
                    rule: self.get_rule("legacy_command_shortcut").unwrap(),
                    message: format!(
                        "Consider migrating Command-based shortcut '{}' to Option key",
                        signature
                    ),
                    entity_id: Some(mapping.id),
                    entity_type: "KeyboardMapping".to_string(),
                });
            }

            if self.is_system_reserved_shortcut(&mapping.shortcut_combination) {
                results.push(ValidationResult {
                    rule: self.get_rule("system_reserved_shortcut").unwrap(),
                    message: format!(
                        "Shortcut '{}' may conflict with system shortcuts",
                        signature
                    ),
                    entity_id: Some(mapping.id),
                    entity_type: "KeyboardMapping".to_string(),
                });
            }
        }

        results
    }

    pub fn validate_application_profiles(
        &self,
        profiles: &[ApplicationProfile],
    ) -> Vec<ValidationResult> {
        let mut results = Vec::new();
        let mut bundle_id_map = HashMap::new();

        for profile in profiles {
            if let Some(existing_id) = bundle_id_map.insert(&profile.bundle_identifier, profile.id)
            {
                if existing_id != profile.id {
                    results.push(ValidationResult {
                        rule: self.get_rule("duplicate_bundle_identifier").unwrap(),
                        message: format!(
                            "Bundle identifier '{}' is used by multiple profiles",
                            profile.bundle_identifier
                        ),
                        entity_id: Some(profile.id),
                        entity_type: "ApplicationProfile".to_string(),
                    });
                }
            }

            if !self.bundle_id_regex.is_match(&profile.bundle_identifier) {
                results.push(ValidationResult {
                    rule: self.get_rule("invalid_bundle_identifier").unwrap(),
                    message: format!(
                        "Invalid bundle identifier format: '{}'",
                        profile.bundle_identifier
                    ),
                    entity_id: Some(profile.id),
                    entity_type: "ApplicationProfile".to_string(),
                });
            }

            if profile.display_name.is_empty() {
                results.push(ValidationResult {
                    rule: self.get_rule("empty_display_name").unwrap(),
                    message: "Application display name cannot be empty".to_string(),
                    entity_id: Some(profile.id),
                    entity_type: "ApplicationProfile".to_string(),
                });
            }
        }

        results
    }

    pub fn validate_monitor_configurations(
        &self,
        configs: &[MonitorConfiguration],
    ) -> Vec<ValidationResult> {
        let mut results = Vec::new();

        for config in configs {
            if config.scale_factor <= 0.0 {
                results.push(ValidationResult {
                    rule: self.get_rule("invalid_scale_factor").unwrap(),
                    message: format!("Scale factor {} must be positive", config.scale_factor),
                    entity_id: Some(config.id),
                    entity_type: "MonitorConfiguration".to_string(),
                });
            }

            if config.scale_factor < 0.5 || config.scale_factor > 3.0 {
                results.push(ValidationResult {
                    rule: self.get_rule("unusual_scale_factor").unwrap(),
                    message: format!(
                        "Unusual scale factor {}, typical range is 0.5-3.0",
                        config.scale_factor
                    ),
                    entity_id: Some(config.id),
                    entity_type: "MonitorConfiguration".to_string(),
                });
            }
        }

        results
    }

    fn validate_cross_references(&self, config: &WorkspaceConfig) -> Vec<ValidationResult> {
        let mut results = Vec::new();

        let pattern_ids: HashSet<_> = config.patterns.iter().map(|p| p.id).collect();
        let workspace_ids: HashSet<_> = config.workspaces.iter().map(|w| w.id).collect();
        let profile_ids: HashSet<_> = config.application_profiles.iter().map(|p| p.id).collect();

        for workspace in &config.workspaces {
            if !pattern_ids.contains(&workspace.tiling_pattern_id) {
                results.push(ValidationResult {
                    rule: self.get_rule("missing_tiling_pattern_reference").unwrap(),
                    message: format!(
                        "Workspace '{}' references non-existent tiling pattern",
                        workspace.name
                    ),
                    entity_id: Some(workspace.id),
                    entity_type: "Workspace".to_string(),
                });
            }
        }

        for rule in &config.window_rules {
            if !workspace_ids.contains(&rule.workspace_id) {
                results.push(ValidationResult {
                    rule: self.get_rule("missing_workspace_reference").unwrap(),
                    message: "Window rule references non-existent workspace".to_string(),
                    entity_id: Some(rule.id),
                    entity_type: "WindowRule".to_string(),
                });
            }

            if let Some(profile_id) = rule.application_profile_id {
                if !profile_ids.contains(&profile_id) {
                    results.push(ValidationResult {
                        rule: self
                            .get_rule("missing_application_profile_reference")
                            .unwrap(),
                        message: "Window rule references non-existent application profile"
                            .to_string(),
                        entity_id: Some(rule.id),
                        entity_type: "WindowRule".to_string(),
                    });
                }
            }
        }

        for mapping in &config.keyboard_mappings {
            if let ActionParameters::WorkspaceId(target_id) = &mapping.parameters {
                if !workspace_ids.contains(target_id) {
                    results.push(ValidationResult {
                        rule: self.get_rule("missing_keyboard_target_reference").unwrap(),
                        message: format!(
                            "Keyboard mapping '{}' references non-existent workspace",
                            mapping.shortcut_combination.to_config_string()
                        ),
                        entity_id: Some(mapping.id),
                        entity_type: "KeyboardMapping".to_string(),
                    });
                }
            }
        }

        for monitor_config in &config.monitor_configs {
            if !workspace_ids.contains(&monitor_config.workspace_id) {
                results.push(ValidationResult {
                    rule: self.get_rule("missing_workspace_reference").unwrap(),
                    message: "Monitor configuration references non-existent workspace".to_string(),
                    entity_id: Some(monitor_config.id),
                    entity_type: "MonitorConfiguration".to_string(),
                });
            }

            if !pattern_ids.contains(&monitor_config.primary_pattern_id) {
                results.push(ValidationResult {
                    rule: self.get_rule("missing_tiling_pattern_reference").unwrap(),
                    message: "Monitor configuration references non-existent primary tiling pattern"
                        .to_string(),
                    entity_id: Some(monitor_config.id),
                    entity_type: "MonitorConfiguration".to_string(),
                });
            }

            if let Some(secondary_id) = monitor_config.secondary_pattern_id {
                if !pattern_ids.contains(&secondary_id) {
                    results.push(ValidationResult {
                        rule: self.get_rule("missing_tiling_pattern_reference").unwrap(),
                        message:
                            "Monitor configuration references non-existent secondary tiling pattern"
                                .to_string(),
                        entity_id: Some(monitor_config.id),
                        entity_type: "MonitorConfiguration".to_string(),
                    });
                }
            }
        }

        results
    }

    fn validate_performance_implications(&self, config: &WorkspaceConfig) -> Vec<ValidationResult> {
        let mut results = Vec::new();

        if config.workspaces.len() > 20 {
            results.push(ValidationResult {
                rule: self.get_rule("high_workspace_count").unwrap(),
                message: format!(
                    "High workspace count ({}) may impact performance",
                    config.workspaces.len()
                ),
                entity_id: None,
                entity_type: "Configuration".to_string(),
            });
        }

        if config.keyboard_mappings.len() > 100 {
            results.push(ValidationResult {
                rule: self.get_rule("high_keyboard_mapping_count").unwrap(),
                message: format!(
                    "High keyboard mapping count ({}) may impact performance",
                    config.keyboard_mappings.len()
                ),
                entity_id: None,
                entity_type: "Configuration".to_string(),
            });
        }

        let total_window_rules = config.window_rules.len();
        if total_window_rules > 200 {
            results.push(ValidationResult {
                rule: self.get_rule("high_window_rule_count").unwrap(),
                message: format!(
                    "High window rule count ({}) may impact window detection performance",
                    total_window_rules
                ),
                entity_id: None,
                entity_type: "Configuration".to_string(),
            });
        }

        results
    }

    fn is_system_reserved_shortcut(&self, combination: &ShortcutCombination) -> bool {
        let signature = combination.to_config_string().to_lowercase();
        let reserved_shortcuts = [
            "cmd+space",
            "cmd+tab",
            "cmd+q",
            "cmd+w",
            "cmd+a",
            "cmd+s",
            "cmd+d",
            "cmd+f",
            "cmd+z",
            "cmd+x",
            "cmd+c",
            "cmd+v",
            "cmd+shift+z",
            "cmd+shift+4",
            "cmd+shift+3",
            "ctrl+space",
            "ctrl+up",
            "ctrl+down",
            "ctrl+left",
            "ctrl+right",
        ];

        reserved_shortcuts
            .iter()
            .any(|reserved| *reserved == signature)
    }

    fn get_rule(&self, name: &str) -> Option<ValidationRule> {
        self.rules.iter().find(|r| r.name == name).cloned()
    }

    fn default_rules() -> Vec<ValidationRule> {
        vec![
            ValidationRule {
                name: "duplicate_workspace_name".to_string(),
                description: "Workspace names must be unique".to_string(),
                severity: ValidationSeverity::Error,
            },
            ValidationRule {
                name: "duplicate_keyboard_shortcut".to_string(),
                description: "Keyboard shortcuts must be unique".to_string(),
                severity: ValidationSeverity::Error,
            },
            ValidationRule {
                name: "empty_workspace_name".to_string(),
                description: "Workspace names cannot be empty".to_string(),
                severity: ValidationSeverity::Error,
            },
            ValidationRule {
                name: "workspace_name_too_long".to_string(),
                description: "Workspace names should not exceed 100 characters".to_string(),
                severity: ValidationSeverity::Error,
            },
            ValidationRule {
                name: "invalid_keyboard_shortcut".to_string(),
                description: "Keyboard shortcuts must follow valid format".to_string(),
                severity: ValidationSeverity::Error,
            },
            ValidationRule {
                name: "legacy_command_shortcut".to_string(),
                description: "Command-based shortcuts should be migrated to Option key".to_string(),
                severity: ValidationSeverity::Warning,
            },
            ValidationRule {
                name: "workspace_description_too_long".to_string(),
                description: "Workspace descriptions should not exceed 500 characters".to_string(),
                severity: ValidationSeverity::Warning,
            },
            ValidationRule {
                name: "duplicate_pattern_name".to_string(),
                description: "Tiling pattern names must be unique".to_string(),
                severity: ValidationSeverity::Error,
            },
            ValidationRule {
                name: "invalid_main_area_ratio".to_string(),
                description: "Main area ratio must be between 0.1 and 0.9".to_string(),
                severity: ValidationSeverity::Error,
            },
            ValidationRule {
                name: "negative_gap_size".to_string(),
                description: "Gap size cannot be negative".to_string(),
                severity: ValidationSeverity::Error,
            },
            ValidationRule {
                name: "negative_window_margin".to_string(),
                description: "Window margin cannot be negative".to_string(),
                severity: ValidationSeverity::Error,
            },
            ValidationRule {
                name: "zero_max_windows".to_string(),
                description: "Maximum windows must be greater than zero".to_string(),
                severity: ValidationSeverity::Error,
            },
            ValidationRule {
                name: "high_max_windows".to_string(),
                description: "High maximum windows count may impact performance".to_string(),
                severity: ValidationSeverity::Warning,
            },
            ValidationRule {
                name: "invalid_window_title_regex".to_string(),
                description: "Window title patterns must be valid regular expressions".to_string(),
                severity: ValidationSeverity::Error,
            },
            ValidationRule {
                name: "negative_z_order".to_string(),
                description: "Z-order priority cannot be negative".to_string(),
                severity: ValidationSeverity::Error,
            },
            ValidationRule {
                name: "negative_fixed_position".to_string(),
                description: "Fixed position coordinates cannot be negative".to_string(),
                severity: ValidationSeverity::Error,
            },
            ValidationRule {
                name: "invalid_fixed_size".to_string(),
                description: "Fixed size dimensions must be positive".to_string(),
                severity: ValidationSeverity::Error,
            },
            ValidationRule {
                name: "duplicate_bundle_identifier".to_string(),
                description: "Bundle identifiers must be unique".to_string(),
                severity: ValidationSeverity::Error,
            },
            ValidationRule {
                name: "invalid_bundle_identifier".to_string(),
                description: "Bundle identifiers must follow valid format".to_string(),
                severity: ValidationSeverity::Error,
            },
            ValidationRule {
                name: "empty_display_name".to_string(),
                description: "Application display names cannot be empty".to_string(),
                severity: ValidationSeverity::Error,
            },
            ValidationRule {
                name: "invalid_scale_factor".to_string(),
                description: "Scale factor must be positive".to_string(),
                severity: ValidationSeverity::Error,
            },
            ValidationRule {
                name: "unusual_scale_factor".to_string(),
                description: "Scale factor is outside typical range".to_string(),
                severity: ValidationSeverity::Warning,
            },
            ValidationRule {
                name: "missing_tiling_pattern_reference".to_string(),
                description: "References to tiling patterns must be valid".to_string(),
                severity: ValidationSeverity::Error,
            },
            ValidationRule {
                name: "missing_workspace_reference".to_string(),
                description: "References to workspaces must be valid".to_string(),
                severity: ValidationSeverity::Error,
            },
            ValidationRule {
                name: "missing_application_profile_reference".to_string(),
                description: "References to application profiles must be valid".to_string(),
                severity: ValidationSeverity::Error,
            },
            ValidationRule {
                name: "missing_keyboard_target_reference".to_string(),
                description: "Keyboard mapping targets must be valid".to_string(),
                severity: ValidationSeverity::Error,
            },
            ValidationRule {
                name: "system_reserved_shortcut".to_string(),
                description: "Shortcut may conflict with system shortcuts".to_string(),
                severity: ValidationSeverity::Warning,
            },
            ValidationRule {
                name: "high_workspace_count".to_string(),
                description: "High workspace count may impact performance".to_string(),
                severity: ValidationSeverity::Info,
            },
            ValidationRule {
                name: "high_keyboard_mapping_count".to_string(),
                description: "High keyboard mapping count may impact performance".to_string(),
                severity: ValidationSeverity::Info,
            },
            ValidationRule {
                name: "high_window_rule_count".to_string(),
                description: "High window rule count may impact performance".to_string(),
                severity: ValidationSeverity::Info,
            },
        ]
    }
}

impl Default for ConfigValidator {
    fn default() -> Self {
        Self::new().expect("Failed to create default ConfigValidator")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::keyboard_mapping::ShortcutCombination;
    use crate::models::tiling_pattern::{LayoutAlgorithm, ResizeBehavior, TilingPattern};
    use crate::models::workspace::{Workspace, WorkspaceState};
    use chrono::Utc;
    use std::str::FromStr;
    use uuid::Uuid;

    #[test]
    fn test_validate_workspace_duplicate_names() {
        let validator = ConfigValidator::new().unwrap();
        let workspaces = vec![
            Workspace {
                id: Uuid::new_v4(),
                name: "Test".to_string(),
                description: None,
                keyboard_shortcut: "opt+1".to_string(),
                tiling_pattern_id: Uuid::new_v4(),
                monitor_assignments: std::collections::HashMap::new(),
                auto_arrange: true,
                created_at: Utc::now(),
                last_used: Some(Utc::now()),
                state: WorkspaceState::default(),
            },
            Workspace {
                id: Uuid::new_v4(),
                name: "Test".to_string(),
                description: None,
                keyboard_shortcut: "opt+2".to_string(),
                tiling_pattern_id: Uuid::new_v4(),
                monitor_assignments: std::collections::HashMap::new(),
                auto_arrange: true,
                created_at: Utc::now(),
                last_used: Some(Utc::now()),
                state: WorkspaceState::default(),
            },
        ];

        let results = validator.validate_workspaces(&workspaces);
        assert!(!results.is_empty());
        assert!(results
            .iter()
            .any(|r| r.rule.name == "duplicate_workspace_name"));
    }

    #[test]
    fn test_validate_legacy_command_shortcuts() {
        let validator = ConfigValidator::new().unwrap();
        let workspaces = vec![Workspace {
            id: Uuid::new_v4(),
            name: "Test".to_string(),
            description: None,
            keyboard_shortcut: "cmd+1".to_string(),
            tiling_pattern_id: Uuid::new_v4(),
            monitor_assignments: std::collections::HashMap::new(),
            auto_arrange: true,
            created_at: Utc::now(),
            last_used: Some(Utc::now()),
            state: WorkspaceState::default(),
        }];

        let results = validator.validate_workspaces(&workspaces);
        assert!(results
            .iter()
            .any(|r| r.rule.name == "legacy_command_shortcut"));
        assert!(results
            .iter()
            .any(|r| r.rule.severity == ValidationSeverity::Warning));
    }

    #[test]
    fn test_validate_tiling_pattern_main_area_ratio() {
        let validator = ConfigValidator::new().unwrap();
        let patterns = vec![TilingPattern {
            id: Uuid::new_v4(),
            name: "Invalid".to_string(),
            layout_algorithm: LayoutAlgorithm::MasterStack,
            main_area_ratio: 0.05, // Invalid: too small
            gap_size: 10,
            window_margin: 5,
            max_windows: 10,
            resize_behavior: ResizeBehavior::Shrink,
        }];

        let results = validator.validate_tiling_patterns(&patterns);
        assert!(results
            .iter()
            .any(|r| r.rule.name == "invalid_main_area_ratio"));
        assert!(results
            .iter()
            .any(|r| r.rule.severity == ValidationSeverity::Error));
    }

    #[test]
    fn test_validate_system_reserved_shortcuts() {
        let validator = ConfigValidator::new().unwrap();
        let cmd_space = ShortcutCombination::from_str("cmd+space").unwrap();
        let cmd_tab = ShortcutCombination::from_str("cmd+tab").unwrap();
        let opt_one = ShortcutCombination::from_str("opt+1").unwrap();

        assert!(validator.is_system_reserved_shortcut(&cmd_space));
        assert!(validator.is_system_reserved_shortcut(&cmd_tab));
        assert!(!validator.is_system_reserved_shortcut(&opt_one));
    }
}
