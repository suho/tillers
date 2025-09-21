use crate::models::workspace::Workspace;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum SimplePersistenceError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] toml::ser::Error),
    #[error("Parse error: {0}")]
    Parse(#[from] toml::de::Error),
}

#[derive(Debug, Clone)]
pub struct SimplePersistenceConfig {
    pub config_dir: PathBuf,
}

impl Default for SimplePersistenceConfig {
    fn default() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let config_dir = home_dir.join(".config").join("tillers");

        Self { config_dir }
    }
}

pub struct SimpleConfigPersistence {
    config: SimplePersistenceConfig,
}

impl SimpleConfigPersistence {
    pub fn new(config: SimplePersistenceConfig) -> Self {
        Self { config }
    }

    pub fn initialize_config_directory(&self) -> Result<(), SimplePersistenceError> {
        if !self.config.config_dir.exists() {
            fs::create_dir_all(&self.config.config_dir)?;
        }

        let workspaces_file = self.config.config_dir.join("workspaces.toml");
        if !workspaces_file.exists() {
            let default_content = self.create_default_workspaces_config()?;
            fs::write(&workspaces_file, default_content)?;
        }

        Ok(())
    }

    pub fn load_workspaces(&self) -> Result<Vec<Workspace>, SimplePersistenceError> {
        let file_path = self.config.config_dir.join("workspaces.toml");

        if !file_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(file_path)?;
        let workspaces: Vec<Workspace> = toml::from_str(&content)?;
        Ok(workspaces)
    }

    pub fn save_workspaces(&self, workspaces: &[Workspace]) -> Result<(), SimplePersistenceError> {
        let file_path = self.config.config_dir.join("workspaces.toml");
        let content = toml::to_string_pretty(&workspaces)?;

        // Atomic write
        let temp_path = file_path.with_extension("tmp");
        fs::write(&temp_path, content)?;
        fs::rename(temp_path, file_path)?;

        Ok(())
    }

    fn create_default_workspaces_config(&self) -> Result<String, SimplePersistenceError> {
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
}

impl Default for SimpleConfigPersistence {
    fn default() -> Self {
        Self::new(SimplePersistenceConfig::default())
    }
}
