use crate::config::parser::{ConfigParser, ConfigParseError, WorkspaceConfig, ConfigFile};
use crate::config::validator::{ConfigValidator, ValidationResult, ValidationSeverity};
use crate::models::{
    workspace::Workspace,
    tiling_pattern::TilingPattern,
    window_rule::WindowRule,
    keyboard_mapping::KeyboardMapping,
    application_profile::ApplicationProfile,
    monitor_configuration::MonitorConfiguration,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum PersistenceError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] toml::ser::Error),
    #[error("Parse error: {0}")]
    ParseError(#[from] ConfigParseError),
    #[error("Validation failed: {validation_errors:?}")]
    ValidationError { validation_errors: Vec<ValidationResult> },
    #[error("Backup error: {message}")]
    BackupError { message: String },
    #[error("Lock error: {message}")]
    LockError { message: String },
    #[error("Migration error: {message}")]
    MigrationError { message: String },
}

#[derive(Debug, Clone)]
pub struct PersistenceConfig {
    pub config_dir: PathBuf,
    pub backup_dir: PathBuf,
    pub max_backups: usize,
    pub enable_validation: bool,
    pub atomic_writes: bool,
    pub file_permissions: u32,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let config_dir = home_dir.join(".config").join("tillers");
        
        Self {
            backup_dir: config_dir.join("backups"),
            config_dir,
            max_backups: 10,
            enable_validation: true,
            atomic_writes: true,
            file_permissions: 0o600, // Read/write for owner only
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct BackupMetadata {
    pub timestamp: u64,
    pub original_file: String,
    pub backup_reason: String,
    pub config_version: String,
}

pub struct ConfigPersistence {
    config: PersistenceConfig,
    parser: ConfigParser,
    validator: ConfigValidator,
}

impl ConfigPersistence {
    pub fn new(config: PersistenceConfig) -> Result<Self, PersistenceError> {
        let parser = ConfigParser::new();
        let validator = ConfigValidator::new()
            .map_err(|e| PersistenceError::ParseError(e))?;

        Ok(Self {
            config,
            parser,
            validator,
        })
    }

    pub fn initialize_config_directory(&self) -> Result<(), PersistenceError> {
        self.ensure_directory_exists(&self.config.config_dir)?;
        self.ensure_directory_exists(&self.config.backup_dir)?;

        let default_files = [
            ("workspaces.toml", self.create_default_workspaces_config()?),
            ("patterns.toml", self.create_default_patterns_config()?),
            ("keybindings.toml", self.create_default_keybindings_config()?),
            ("applications.toml", self.create_default_applications_config()?),
            ("monitors.toml", self.create_default_monitors_config()?),
        ];

        for (filename, content) in &default_files {
            let file_path = self.config.config_dir.join(filename);
            if !file_path.exists() {
                self.write_file_atomic(&file_path, content)?;
            }
        }

        Ok(())
    }

    pub fn load_full_config(&mut self) -> Result<WorkspaceConfig, PersistenceError> {
        let workspaces = self.load_workspaces()?;
        let patterns = self.load_tiling_patterns()?;
        let window_rules = self.load_window_rules()?;
        let keyboard_mappings = self.load_keyboard_mappings()?;
        let application_profiles = self.load_application_profiles()?;
        let monitor_configs = self.load_monitor_configurations()?;

        let config = WorkspaceConfig {
            workspaces,
            patterns,
            window_rules,
            keyboard_mappings,
            application_profiles,
            monitor_configs,
        };

        if self.config.enable_validation {
            self.validate_config(&config)?;
        }

        Ok(config)
    }

    pub fn save_full_config(&mut self, config: &WorkspaceConfig) -> Result<(), PersistenceError> {
        if self.config.enable_validation {
            self.validate_config(config)?;
        }

        self.backup_existing_files("full_config_save")?;

        self.save_workspaces(&config.workspaces)?;
        self.save_tiling_patterns(&config.patterns)?;
        self.save_window_rules(&config.window_rules)?;
        self.save_keyboard_mappings(&config.keyboard_mappings)?;
        self.save_application_profiles(&config.application_profiles)?;
        self.save_monitor_configurations(&config.monitor_configs)?;

        self.cleanup_old_backups()?;

        Ok(())
    }

    pub fn load_workspaces(&mut self) -> Result<Vec<Workspace>, PersistenceError> {
        let file_path = self.config.config_dir.join("workspaces.toml");
        let content = self.read_file_with_lock(&file_path)?;
        Ok(self.parser.parse_workspaces_toml(&content)?)
    }

    pub fn save_workspaces(&mut self, workspaces: &[Workspace]) -> Result<(), PersistenceError> {
        let file_path = self.config.config_dir.join("workspaces.toml");
        self.backup_file(&file_path, "workspace_update")?;
        
        let content = toml::to_string_pretty(&workspaces)?;
        self.write_file_atomic(&file_path, &content)?;
        
        Ok(())
    }

    pub fn load_tiling_patterns(&mut self) -> Result<Vec<TilingPattern>, PersistenceError> {
        let file_path = self.config.config_dir.join("patterns.toml");
        let content = self.read_file_with_lock(&file_path)?;
        Ok(self.parser.parse_patterns_toml(&content)?)
    }

    pub fn save_tiling_patterns(&mut self, patterns: &[TilingPattern]) -> Result<(), PersistenceError> {
        let file_path = self.config.config_dir.join("patterns.toml");
        self.backup_file(&file_path, "pattern_update")?;
        
        let content = toml::to_string_pretty(&patterns)?;
        self.write_file_atomic(&file_path, &content)?;
        
        Ok(())
    }

    pub fn load_window_rules(&mut self) -> Result<Vec<WindowRule>, PersistenceError> {
        let file_path = self.config.config_dir.join("window_rules.toml");
        if !file_path.exists() {
            return Ok(Vec::new());
        }
        
        let content = self.read_file_with_lock(&file_path)?;
        let rules: Vec<WindowRule> = toml::from_str(&content)
            .map_err(|e| ConfigParseError::TomlError(e))?;
        Ok(rules)
    }

    pub fn save_window_rules(&mut self, rules: &[WindowRule]) -> Result<(), PersistenceError> {
        let file_path = self.config.config_dir.join("window_rules.toml");
        self.backup_file(&file_path, "window_rules_update")?;
        
        let content = toml::to_string_pretty(&rules)?;
        self.write_file_atomic(&file_path, &content)?;
        
        Ok(())
    }

    pub fn load_keyboard_mappings(&mut self) -> Result<Vec<KeyboardMapping>, PersistenceError> {
        let file_path = self.config.config_dir.join("keybindings.toml");
        let content = self.read_file_with_lock(&file_path)?;
        Ok(self.parser.parse_keybindings_toml(&content)?)
    }

    pub fn save_keyboard_mappings(&mut self, mappings: &[KeyboardMapping]) -> Result<(), PersistenceError> {
        let file_path = self.config.config_dir.join("keybindings.toml");
        self.backup_file(&file_path, "keybindings_update")?;
        
        let content = toml::to_string_pretty(&mappings)?;
        self.write_file_atomic(&file_path, &content)?;
        
        Ok(())
    }

    pub fn load_application_profiles(&mut self) -> Result<Vec<ApplicationProfile>, PersistenceError> {
        let file_path = self.config.config_dir.join("applications.toml");
        let content = self.read_file_with_lock(&file_path)?;
        Ok(self.parser.parse_applications_toml(&content)?)
    }

    pub fn save_application_profiles(&mut self, profiles: &[ApplicationProfile]) -> Result<(), PersistenceError> {
        let file_path = self.config.config_dir.join("applications.toml");
        self.backup_file(&file_path, "applications_update")?;
        
        let content = toml::to_string_pretty(&profiles)?;
        self.write_file_atomic(&file_path, &content)?;
        
        Ok(())
    }

    pub fn load_monitor_configurations(&mut self) -> Result<Vec<MonitorConfiguration>, PersistenceError> {
        let file_path = self.config.config_dir.join("monitors.toml");
        let content = self.read_file_with_lock(&file_path)?;
        Ok(self.parser.parse_monitors_toml(&content)?)
    }

    pub fn save_monitor_configurations(&mut self, configs: &[MonitorConfiguration]) -> Result<(), PersistenceError> {
        let file_path = self.config.config_dir.join("monitors.toml");
        self.backup_file(&file_path, "monitors_update")?;
        
        let content = toml::to_string_pretty(&configs)?;
        self.write_file_atomic(&file_path, &content)?;
        
        Ok(())
    }

    pub fn export_config(&mut self, export_path: &Path) -> Result<(), PersistenceError> {
        let config = self.load_full_config()?;
        
        let config_file = ConfigFile {
            version: "1.0".to_string(),
            config,
        };
        
        let content = toml::to_string_pretty(&config_file)?;
        self.write_file_atomic(export_path, &content)?;
        
        Ok(())
    }

    pub fn import_config(&mut self, import_path: &Path) -> Result<Vec<String>, PersistenceError> {
        let content = fs::read_to_string(import_path)?;
        let config_file: ConfigFile = toml::from_str(&content)
            .map_err(|e| ConfigParseError::TomlError(e))?;

        if self.config.enable_validation {
            self.validate_config(&config_file.config)?;
        }

        self.backup_existing_files("config_import")?;
        self.save_full_config(&config_file.config)?;
        
        Ok(self.parser.get_migration_warnings().to_vec())
    }

    pub fn get_migration_warnings(&self) -> &[String] {
        self.parser.get_migration_warnings()
    }

    fn validate_config(&self, config: &WorkspaceConfig) -> Result<(), PersistenceError> {
        let validation_results = self.validator.validate_full_config(config);
        
        let errors: Vec<_> = validation_results
            .iter()
            .filter(|r| r.rule.severity == ValidationSeverity::Error)
            .cloned()
            .collect();

        if !errors.is_empty() {
            return Err(PersistenceError::ValidationError {
                validation_errors: errors,
            });
        }

        Ok(())
    }

    fn ensure_directory_exists(&self, dir: &Path) -> Result<(), PersistenceError> {
        if !dir.exists() {
            fs::create_dir_all(dir)?;
            
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let permissions = fs::Permissions::from_mode(0o700); // rwx for owner only
                fs::set_permissions(dir, permissions)?;
            }
        }
        Ok(())
    }

    fn read_file_with_lock(&self, file_path: &Path) -> Result<String, PersistenceError> {
        if !file_path.exists() {
            return Ok(String::new());
        }

        let content = fs::read_to_string(file_path)?;
        Ok(content)
    }

    fn write_file_atomic(&self, file_path: &Path, content: &str) -> Result<(), PersistenceError> {
        if self.config.atomic_writes {
            let temp_path = file_path.with_extension("tmp");
            
            fs::write(&temp_path, content)?;
            
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let permissions = fs::Permissions::from_mode(self.config.file_permissions);
                fs::set_permissions(&temp_path, permissions)?;
            }
            
            fs::rename(temp_path, file_path)?;
        } else {
            fs::write(file_path, content)?;
            
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let permissions = fs::Permissions::from_mode(self.config.file_permissions);
                fs::set_permissions(file_path, permissions)?;
            }
        }
        
        Ok(())
    }

    fn backup_file(&self, file_path: &Path, reason: &str) -> Result<(), PersistenceError> {
        if !file_path.exists() {
            return Ok(());
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let filename = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| PersistenceError::BackupError {
                message: "Invalid file name".to_string(),
            })?;

        let backup_filename = format!("{}.{}.backup", filename, timestamp);
        let backup_path = self.config.backup_dir.join(&backup_filename);

        fs::copy(file_path, &backup_path)?;

        let metadata = BackupMetadata {
            timestamp,
            original_file: filename.to_string(),
            backup_reason: reason.to_string(),
            config_version: "1.0".to_string(),
        };

        let metadata_path = backup_path.with_extension("metadata");
        let metadata_content = toml::to_string_pretty(&metadata)?;
        fs::write(metadata_path, metadata_content)?;

        Ok(())
    }

    fn backup_existing_files(&self, reason: &str) -> Result<(), PersistenceError> {
        let config_files = [
            "workspaces.toml",
            "patterns.toml",
            "keybindings.toml",
            "applications.toml",
            "monitors.toml",
            "window_rules.toml",
        ];

        for filename in &config_files {
            let file_path = self.config.config_dir.join(filename);
            self.backup_file(&file_path, reason)?;
        }

        Ok(())
    }

    fn cleanup_old_backups(&self) -> Result<(), PersistenceError> {
        let mut backup_files = Vec::new();

        for entry in fs::read_dir(&self.config.backup_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("backup") {
                if let Some(metadata) = fs::metadata(&path).ok() {
                    if let Ok(modified) = metadata.modified() {
                        backup_files.push((path, modified));
                    }
                }
            }
        }

        backup_files.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by modification time, newest first

        if backup_files.len() > self.config.max_backups {
            for (old_backup, _) in backup_files.iter().skip(self.config.max_backups) {
                fs::remove_file(old_backup)?;
                
                let metadata_path = old_backup.with_extension("metadata");
                if metadata_path.exists() {
                    fs::remove_file(metadata_path)?;
                }
            }
        }

        Ok(())
    }

    fn create_default_workspaces_config(&self) -> Result<String, PersistenceError> {
        let default_workspace = Workspace {
            id: Uuid::new_v4(),
            name: "Default".to_string(),
            description: Some("Default workspace".to_string()),
            keyboard_shortcut: "opt+1".to_string(),
            tiling_pattern_id: Uuid::new_v4(),
            monitor_assignments: std::collections::HashMap::new(),
            auto_arrange: true,
            created_at: chrono::Utc::now(),
            last_used: Some(chrono::Utc::now()),
            state: crate::models::workspace::WorkspaceState::default(),
        };

        let workspaces = vec![default_workspace];
        Ok(toml::to_string_pretty(&workspaces)?)
    }

    fn create_default_patterns_config(&self) -> Result<String, PersistenceError> {
        use crate::models::tiling_pattern::{LayoutAlgorithm, ResizeBehavior};
        
        let default_pattern = TilingPattern {
            id: Uuid::new_v4(),
            name: "Two Column".to_string(),
            layout_algorithm: LayoutAlgorithm::Columns,
            main_area_ratio: 0.6,
            gap_size: 10,
            window_margin: 5,
            max_windows: 10,
            resize_behavior: ResizeBehavior::Shrink,
        };

        let patterns = vec![default_pattern];
        Ok(toml::to_string_pretty(&patterns)?)
    }

    fn create_default_keybindings_config(&self) -> Result<String, PersistenceError> {
        let mappings: Vec<KeyboardMapping> = Vec::new();
        Ok(toml::to_string_pretty(&mappings)?)
    }

    fn create_default_applications_config(&self) -> Result<String, PersistenceError> {
        let profiles: Vec<ApplicationProfile> = Vec::new();
        Ok(toml::to_string_pretty(&profiles)?)
    }

    fn create_default_monitors_config(&self) -> Result<String, PersistenceError> {
        let configs: Vec<MonitorConfiguration> = Vec::new();
        Ok(toml::to_string_pretty(&configs)?)
    }
}

impl Default for ConfigPersistence {
    fn default() -> Self {
        Self::new(PersistenceConfig::default())
            .expect("Failed to create default ConfigPersistence")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_initialize_config_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().join("test_config");
        let backup_dir = config_dir.join("backups");
        
        let persistence_config = PersistenceConfig {
            config_dir: config_dir.clone(),
            backup_dir,
            ..PersistenceConfig::default()
        };

        let persistence = ConfigPersistence::new(persistence_config).unwrap();
        persistence.initialize_config_directory().unwrap();

        assert!(config_dir.exists());
        assert!(config_dir.join("workspaces.toml").exists());
        assert!(config_dir.join("patterns.toml").exists());
        assert!(config_dir.join("keybindings.toml").exists());
        assert!(config_dir.join("applications.toml").exists());
        assert!(config_dir.join("monitors.toml").exists());
    }

    #[test]
    fn test_atomic_write() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().join("test_config");
        
        let persistence_config = PersistenceConfig {
            config_dir: config_dir.clone(),
            atomic_writes: true,
            ..PersistenceConfig::default()
        };

        let persistence = ConfigPersistence::new(persistence_config).unwrap();
        let test_file = config_dir.join("test.toml");
        
        fs::create_dir_all(&config_dir).unwrap();
        
        let test_content = "test = \"content\"";
        persistence.write_file_atomic(&test_file, test_content).unwrap();
        
        let read_content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(read_content, test_content);
    }

    #[test]
    fn test_backup_and_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().join("test_config");
        let backup_dir = config_dir.join("backups");
        
        let persistence_config = PersistenceConfig {
            config_dir: config_dir.clone(),
            backup_dir: backup_dir.clone(),
            max_backups: 2,
            ..PersistenceConfig::default()
        };

        let persistence = ConfigPersistence::new(persistence_config).unwrap();
        persistence.initialize_config_directory().unwrap();
        
        let test_file = config_dir.join("test.toml");
        fs::write(&test_file, "original content").unwrap();

        // Create multiple backups
        for i in 0..5 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            fs::write(&test_file, format!("content {}", i)).unwrap();
            persistence.backup_file(&test_file, "test").unwrap();
        }

        persistence.cleanup_old_backups().unwrap();

        let backup_count = fs::read_dir(&backup_dir)
            .unwrap()
            .filter(|entry| {
                entry.as_ref().unwrap().path()
                    .extension()
                    .and_then(|s| s.to_str()) == Some("backup")
            })
            .count();

        assert_eq!(backup_count, 2);
    }
}