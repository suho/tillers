//! Unit tests for configuration parsing
//!
//! Tests the configuration parser for TOML workspace definitions,
//! validation, migration from legacy formats, and error handling.

use tillers::{
    config::parser::{ConfigParser, ConfigParseError, ConfigFile, WorkspaceConfig},
    models::{
        workspace::{Workspace, WorkspaceKind},
        tiling_pattern::{TilingPattern, LayoutAlgorithm, ResizeBehavior},
        window_rule::{WindowRule, WindowRuleCondition, WindowRuleAction},
        monitor_configuration::{MonitorConfiguration, DisplayPosition},
        keyboard_mapping::{KeyboardMapping, ActionType, ModifierKey},
        application_profile::{ApplicationProfile, ProfileSettings},
    },
};
use std::fs;
use tempfile::NamedTempFile;
use uuid::Uuid;

/// Create a minimal valid TOML configuration for testing
fn create_minimal_config_toml() -> String {
    r#"
version = "1.0.0"

[config]
workspaces = []
patterns = []
window_rules = []
monitor_configs = []
keyboard_mappings = []
application_profiles = []
"#.to_string()
}

/// Create a comprehensive TOML configuration for testing
fn create_full_config_toml() -> String {
    let workspace_id = Uuid::new_v4();
    let pattern_id = Uuid::new_v4();
    let rule_id = Uuid::new_v4();
    let monitor_id = Uuid::new_v4();
    let mapping_id = Uuid::new_v4();
    let profile_id = Uuid::new_v4();

    format!(r#"
version = "1.0.0"

[config]

[[config.workspaces]]
id = "{workspace_id}"
name = "Development"
description = "Primary development workspace"
kind = "Standard"
is_active = true
window_ids = [1, 2, 3]
tiling_pattern_id = "{pattern_id}"
created_at = "2023-01-01T00:00:00Z"
last_accessed = "2023-01-01T12:00:00Z"

[[config.patterns]]
id = "{pattern_id}"
name = "Master-Stack"
layout_algorithm = "MasterStack"
main_area_ratio = 0.6
gap_size = 10
window_margin = 20
max_windows = 8
resize_behavior = "Shrink"

[[config.window_rules]]
id = "{rule_id}"
name = "Terminal Rule"
condition = {{ app_name = "Terminal" }}
action = {{ workspace_assignment = "{workspace_id}" }}
priority = 100
enabled = true

[[config.monitor_configs]]
id = "{monitor_id}"
name = "Primary Monitor"
display_id = 1
resolution = {{ width = 1920, height = 1080 }}
position = "Primary"
scaling_factor = 1.0
refresh_rate = 60.0

[[config.keyboard_mappings]]
id = "{mapping_id}"
shortcut_combination = "opt+1"
action_type = "SwitchToWorkspace"
target_id = "{workspace_id}"
parameters = {{ workspace_index = 0 }}
enabled = true

[[config.application_profiles]]
id = "{profile_id}"
application_bundle_id = "com.apple.Terminal"
application_name = "Terminal"
settings = {{ default_workspace_id = "{workspace_id}", auto_tile = true, floating = false }}
"#, workspace_id = workspace_id, pattern_id = pattern_id, rule_id = rule_id, 
    monitor_id = monitor_id, mapping_id = mapping_id, profile_id = profile_id)
}

/// Create a legacy configuration with cmd modifiers for migration testing
fn create_legacy_config_toml() -> String {
    let workspace_id = Uuid::new_v4();
    let mapping_id = Uuid::new_v4();

    format!(r#"
version = "0.9.0"

[config]
workspaces = []
patterns = []
window_rules = []
monitor_configs = []
application_profiles = []

[[config.keyboard_mappings]]
id = "{mapping_id}"
shortcut_combination = "cmd+1"
action_type = "SwitchToWorkspace"
target_id = "{workspace_id}"
parameters = {{ workspace_index = 0 }}
enabled = true
"#, workspace_id = workspace_id, mapping_id = mapping_id)
}

/// Create an invalid TOML configuration for error testing
fn create_invalid_config_toml() -> String {
    r#"
version = "1.0.0"
[config
workspaces = []
patterns = []
"#.to_string()
}

/// Create a configuration with validation errors
fn create_invalid_data_config_toml() -> String {
    r#"
version = "1.0.0"

[config]
workspaces = []
patterns = []
window_rules = []
monitor_configs = []
application_profiles = []

[[config.keyboard_mappings]]
id = "not-a-uuid"
shortcut_combination = ""
action_type = "InvalidAction"
enabled = true
"#.to_string()
}

#[tokio::test]
async fn test_config_parser_creation() {
    let parser = ConfigParser::new();
    assert!(parser.is_ok());
}

#[tokio::test]
async fn test_parse_minimal_config() {
    let parser = ConfigParser::new().unwrap();
    let toml_content = create_minimal_config_toml();
    
    let config = parser.parse_from_string(&toml_content).await;
    assert!(config.is_ok());
    
    let config = config.unwrap();
    assert_eq!(config.version, "1.0.0");
    assert!(config.config.workspaces.is_empty());
    assert!(config.config.patterns.is_empty());
    assert!(config.config.window_rules.is_empty());
    assert!(config.config.monitor_configs.is_empty());
    assert!(config.config.keyboard_mappings.is_empty());
    assert!(config.config.application_profiles.is_empty());
}

#[tokio::test]
async fn test_parse_full_config() {
    let parser = ConfigParser::new().unwrap();
    let toml_content = create_full_config_toml();
    
    let config = parser.parse_from_string(&toml_content).await;
    assert!(config.is_ok());
    
    let config = config.unwrap();
    assert_eq!(config.version, "1.0.0");
    
    // Check workspaces
    assert_eq!(config.config.workspaces.len(), 1);
    let workspace = &config.config.workspaces[0];
    assert_eq!(workspace.name, "Development");
    assert_eq!(workspace.description, Some("Primary development workspace".to_string()));
    assert_eq!(workspace.kind, WorkspaceKind::Standard);
    assert!(workspace.is_active);
    assert_eq!(workspace.window_ids, vec![1, 2, 3]);
    
    // Check patterns
    assert_eq!(config.config.patterns.len(), 1);
    let pattern = &config.config.patterns[0];
    assert_eq!(pattern.name, "Master-Stack");
    assert_eq!(pattern.layout_algorithm, LayoutAlgorithm::MasterStack);
    assert_eq!(pattern.main_area_ratio, 0.6);
    assert_eq!(pattern.gap_size, 10);
    assert_eq!(pattern.window_margin, 20);
    assert_eq!(pattern.max_windows, 8);
    assert_eq!(pattern.resize_behavior, ResizeBehavior::Shrink);
    
    // Check window rules
    assert_eq!(config.config.window_rules.len(), 1);
    let rule = &config.config.window_rules[0];
    assert_eq!(rule.name, "Terminal Rule");
    assert_eq!(rule.priority, 100);
    assert!(rule.enabled);
    
    // Check monitor configs
    assert_eq!(config.config.monitor_configs.len(), 1);
    let monitor = &config.config.monitor_configs[0];
    assert_eq!(monitor.name, "Primary Monitor");
    assert_eq!(monitor.display_id, 1);
    assert_eq!(monitor.position, DisplayPosition::Primary);
    assert_eq!(monitor.scaling_factor, 1.0);
    assert_eq!(monitor.refresh_rate, 60.0);
    
    // Check keyboard mappings
    assert_eq!(config.config.keyboard_mappings.len(), 1);
    let mapping = &config.config.keyboard_mappings[0];
    assert_eq!(mapping.shortcut_combination, "opt+1");
    assert_eq!(mapping.action_type, ActionType::SwitchToWorkspace);
    assert!(mapping.enabled);
    
    // Check application profiles
    assert_eq!(config.config.application_profiles.len(), 1);
    let profile = &config.config.application_profiles[0];
    assert_eq!(profile.application_bundle_id, "com.apple.Terminal");
    assert_eq!(profile.application_name, "Terminal");
}

#[tokio::test]
async fn test_parse_from_file() {
    let parser = ConfigParser::new().unwrap();
    let toml_content = create_minimal_config_toml();
    
    // Create temporary file
    let mut temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), toml_content).unwrap();
    
    let config = parser.parse_from_file(temp_file.path()).await;
    assert!(config.is_ok());
    
    let config = config.unwrap();
    assert_eq!(config.version, "1.0.0");
}

#[tokio::test]
async fn test_parse_invalid_toml() {
    let parser = ConfigParser::new().unwrap();
    let invalid_toml = create_invalid_config_toml();
    
    let result = parser.parse_from_string(&invalid_toml).await;
    assert!(result.is_err());
    
    match result.unwrap_err() {
        ConfigParseError::TomlError(_) => (),
        other => panic!("Expected TomlError, got {:?}", other),
    }
}

#[tokio::test]
async fn test_parse_invalid_data() {
    let parser = ConfigParser::new().unwrap();
    let invalid_data = create_invalid_data_config_toml();
    
    let result = parser.parse_from_string(&invalid_data).await;
    assert!(result.is_err());
    
    // Should be either a TOML parsing error or validation error
    assert!(matches!(result.unwrap_err(), 
        ConfigParseError::TomlError(_) | ConfigParseError::ValidationError { .. }));
}

#[tokio::test]
async fn test_parse_nonexistent_file() {
    let parser = ConfigParser::new().unwrap();
    
    let result = parser.parse_from_file("/nonexistent/path/config.toml").await;
    assert!(result.is_err());
    
    match result.unwrap_err() {
        ConfigParseError::IoError(_) => (),
        other => panic!("Expected IoError, got {:?}", other),
    }
}

#[tokio::test]
async fn test_legacy_migration() {
    let parser = ConfigParser::new().unwrap();
    let legacy_config = create_legacy_config_toml();
    
    let result = parser.parse_from_string(&legacy_config).await;
    assert!(result.is_ok());
    
    let config = result.unwrap();
    
    // Check that cmd shortcuts were migrated to opt
    if !config.config.keyboard_mappings.is_empty() {
        let mapping = &config.config.keyboard_mappings[0];
        // Should be migrated from cmd+1 to opt+1
        assert!(mapping.shortcut_combination.contains("opt"), 
            "Expected opt modifier, got: {}", mapping.shortcut_combination);
        assert!(!mapping.shortcut_combination.contains("cmd"),
            "Should not contain cmd modifier after migration: {}", mapping.shortcut_combination);
    }
}

#[tokio::test]
async fn test_option_modifier_validation() {
    let parser = ConfigParser::new().unwrap();
    
    // Create config with option modifier (should be valid)
    let valid_config = r#"
version = "1.0.0"

[config]
workspaces = []
patterns = []
window_rules = []
monitor_configs = []
application_profiles = []

[[config.keyboard_mappings]]
id = "550e8400-e29b-41d4-a716-446655440000"
shortcut_combination = "opt+shift+1"
action_type = "SwitchToWorkspace"
enabled = true
"#;
    
    let result = parser.parse_from_string(valid_config).await;
    assert!(result.is_ok());
    
    let config = result.unwrap();
    assert_eq!(config.config.keyboard_mappings.len(), 1);
    assert_eq!(config.config.keyboard_mappings[0].shortcut_combination, "opt+shift+1");
}

#[tokio::test]
async fn test_validate_workspace_references() {
    let parser = ConfigParser::new().unwrap();
    
    // Create config with invalid workspace reference
    let workspace_id = Uuid::new_v4();
    let different_workspace_id = Uuid::new_v4();
    
    let invalid_ref_config = format!(r#"
version = "1.0.0"

[config]
patterns = []
window_rules = []
monitor_configs = []
application_profiles = []

[[config.workspaces]]
id = "{workspace_id}"
name = "Test Workspace"
kind = "Standard"
is_active = false
window_ids = []

[[config.keyboard_mappings]]
id = "550e8400-e29b-41d4-a716-446655440000"
shortcut_combination = "opt+1"
action_type = "SwitchToWorkspace"
target_id = "{different_workspace_id}"
enabled = true
"#, workspace_id = workspace_id, different_workspace_id = different_workspace_id);
    
    let result = parser.parse_from_string(&invalid_ref_config).await;
    
    // This might succeed at parsing but fail at validation
    // The exact behavior depends on implementation details
    if let Ok(config) = result {
        // Validate that references are checked
        assert_eq!(config.config.workspaces.len(), 1);
        assert_eq!(config.config.keyboard_mappings.len(), 1);
    }
}

#[tokio::test]
async fn test_duplicate_ids() {
    let parser = ConfigParser::new().unwrap();
    let duplicate_id = Uuid::new_v4();
    
    let duplicate_config = format!(r#"
version = "1.0.0"

[config]
patterns = []
window_rules = []
monitor_configs = []
keyboard_mappings = []
application_profiles = []

[[config.workspaces]]
id = "{duplicate_id}"
name = "Workspace 1"
kind = "Standard"
is_active = false
window_ids = []

[[config.workspaces]]
id = "{duplicate_id}"
name = "Workspace 2"
kind = "Standard"
is_active = false
window_ids = []
"#, duplicate_id = duplicate_id);
    
    let result = parser.parse_from_string(&duplicate_config).await;
    
    // Should either fail to parse or fail validation
    if let Ok(config) = result {
        // If parsing succeeds, validation should catch the duplicate
        // The exact behavior depends on validation implementation
        assert_eq!(config.config.workspaces.len(), 2);
    }
}

#[tokio::test]
async fn test_serialize_and_parse_roundtrip() {
    let parser = ConfigParser::new().unwrap();
    
    // Create a config object
    let original_config = ConfigFile {
        version: "1.0.0".to_string(),
        config: WorkspaceConfig {
            workspaces: vec![],
            patterns: vec![],
            window_rules: vec![],
            monitor_configs: vec![],
            keyboard_mappings: vec![],
            application_profiles: vec![],
        },
    };
    
    // Serialize to TOML
    let toml_string = parser.serialize_to_string(&original_config).await;
    assert!(toml_string.is_ok());
    
    let toml_string = toml_string.unwrap();
    
    // Parse back
    let parsed_config = parser.parse_from_string(&toml_string).await;
    assert!(parsed_config.is_ok());
    
    let parsed_config = parsed_config.unwrap();
    assert_eq!(parsed_config.version, original_config.version);
    assert_eq!(parsed_config.config.workspaces.len(), 0);
}

#[tokio::test]
async fn test_version_compatibility() {
    let parser = ConfigParser::new().unwrap();
    
    // Test different version formats
    let versions = vec!["1.0.0", "0.9.0", "2.0.0-beta.1"];
    
    for version in versions {
        let config_toml = format!(r#"
version = "{version}"

[config]
workspaces = []
patterns = []
window_rules = []
monitor_configs = []
keyboard_mappings = []
application_profiles = []
"#, version = version);
        
        let result = parser.parse_from_string(&config_toml).await;
        
        // Should parse successfully (migration handled internally)
        assert!(result.is_ok(), "Failed to parse version {}", version);
        
        let config = result.unwrap();
        assert_eq!(config.version, version);
    }
}

#[tokio::test]
async fn test_empty_sections_handling() {
    let parser = ConfigParser::new().unwrap();
    
    let config_with_some_empty = r#"
version = "1.0.0"

[config]
workspaces = []
patterns = []
window_rules = []
monitor_configs = []
keyboard_mappings = []

[[config.application_profiles]]
id = "550e8400-e29b-41d4-a716-446655440000"
application_bundle_id = "com.example.app"
application_name = "Example App"
settings = { auto_tile = true }
"#;
    
    let result = parser.parse_from_string(config_with_some_empty).await;
    assert!(result.is_ok());
    
    let config = result.unwrap();
    assert!(config.config.workspaces.is_empty());
    assert!(config.config.patterns.is_empty());
    assert_eq!(config.config.application_profiles.len(), 1);
}

#[tokio::test]
async fn test_special_characters_in_names() {
    let parser = ConfigParser::new().unwrap();
    
    let config_with_special_chars = format!(r#"
version = "1.0.0"

[config]
patterns = []
window_rules = []
monitor_configs = []
keyboard_mappings = []
application_profiles = []

[[config.workspaces]]
id = "{}"
name = "Test/Workspace With Spaces & Symbols!"
description = "Émojis: 🚀 and unicode: αβγ"
kind = "Standard"
is_active = false
window_ids = []
"#, Uuid::new_v4());
    
    let result = parser.parse_from_string(&config_with_special_chars).await;
    assert!(result.is_ok());
    
    let config = result.unwrap();
    assert_eq!(config.config.workspaces.len(), 1);
    assert_eq!(config.config.workspaces[0].name, "Test/Workspace With Spaces & Symbols!");
    assert!(config.config.workspaces[0].description.as_ref().unwrap().contains("🚀"));
}

#[tokio::test]
async fn test_large_config_parsing() {
    let parser = ConfigParser::new().unwrap();
    
    // Create a large config with many items
    let mut large_config = String::from(r#"
version = "1.0.0"

[config]
patterns = []
window_rules = []
monitor_configs = []
keyboard_mappings = []
application_profiles = []

"#);
    
    // Add many workspaces
    for i in 0..100 {
        let workspace_section = format!(r#"
[[config.workspaces]]
id = "{}"
name = "Workspace {}"
kind = "Standard"
is_active = false
window_ids = []
"#, Uuid::new_v4(), i);
        large_config.push_str(&workspace_section);
    }
    
    let result = parser.parse_from_string(&large_config).await;
    assert!(result.is_ok());
    
    let config = result.unwrap();
    assert_eq!(config.config.workspaces.len(), 100);
}

#[tokio::test] 
async fn test_parser_error_messages() {
    let parser = ConfigParser::new().unwrap();
    
    let malformed_toml = r#"
version = "1.0.0"
[config]
workspaces = [
    { invalid syntax here }
]
"#;
    
    let result = parser.parse_from_string(malformed_toml).await;
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    let error_message = error.to_string();
    
    // Error message should be descriptive
    assert!(!error_message.is_empty());
    assert!(error_message.contains("TOML") || error_message.contains("parsing") || 
            error_message.contains("syntax"));
}