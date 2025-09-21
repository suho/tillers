use crate::config::simple_persistence::{
    SimpleConfigPersistence, SimplePersistenceConfig, SimplePersistenceError,
};
use crate::models::{Workspace, WorkspaceCreateRequest};
use crate::{Result, TilleRSError};
use std::collections::{hash_map::Entry, HashMap};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;

type WorkspaceEventListener = Box<dyn Fn(WorkspaceEvent) + Send + Sync>;

/// Events that can be triggered by workspace operations
#[derive(Debug, Clone)]
pub enum WorkspaceEvent {
    /// Workspace was created
    WorkspaceCreated { workspace: Workspace },
    /// Workspace was updated
    WorkspaceUpdated { workspace: Workspace },
    /// Workspace was deleted
    WorkspaceDeleted { workspace_id: Uuid },
    /// Active workspace changed
    WorkspaceActivated {
        workspace: Workspace,
        previous: Option<Uuid>,
    },
    /// Workspace configuration changed
    ConfigurationChanged { workspace_id: Uuid },
}

/// Configuration for workspace manager behavior
#[derive(Debug, Clone)]
pub struct WorkspaceManagerConfig {
    /// Maximum number of workspaces allowed
    pub max_workspaces: usize,
    /// Whether to auto-save workspace changes
    pub auto_save: bool,
    /// Auto-save interval in seconds
    pub auto_save_interval: u64,
    /// Whether to restore last active workspace on startup
    pub restore_last_active: bool,
    /// Performance monitoring enabled
    pub performance_monitoring: bool,
}

impl Default for WorkspaceManagerConfig {
    fn default() -> Self {
        WorkspaceManagerConfig {
            max_workspaces: 20,
            auto_save: true,
            auto_save_interval: 30,
            restore_last_active: true,
            performance_monitoring: true,
        }
    }
}

/// Manages workspace CRUD operations and switching
pub struct WorkspaceManager {
    /// All workspaces indexed by ID
    workspaces: Arc<RwLock<HashMap<Uuid, Workspace>>>,
    /// Currently active workspace ID
    active_workspace_id: Arc<RwLock<Option<Uuid>>>,
    /// Event listeners
    event_listeners: Arc<Mutex<Vec<WorkspaceEventListener>>>,
    /// Configuration
    config: WorkspaceManagerConfig,
    /// Performance metrics
    metrics: Arc<RwLock<WorkspaceMetrics>>,
    /// Configuration persistence
    persistence: Arc<Mutex<SimpleConfigPersistence>>,
}

/// Performance metrics for workspace operations
#[derive(Debug, Default)]
pub struct WorkspaceMetrics {
    /// Total workspace switch count
    pub switch_count: u64,
    /// Average switch time in milliseconds
    pub avg_switch_time_ms: f64,
    /// Last switch time in milliseconds
    pub last_switch_time_ms: f64,
    /// Total workspaces created
    pub created_count: u64,
    /// Total workspaces deleted
    pub deleted_count: u64,
    /// Error count
    pub error_count: u64,
}

impl WorkspaceManager {
    /// Create a new workspace manager
    pub fn new(config: WorkspaceManagerConfig) -> Result<Self> {
        let persistence_config = SimplePersistenceConfig::default();
        let persistence = SimpleConfigPersistence::new(persistence_config);

        Ok(WorkspaceManager {
            workspaces: Arc::new(RwLock::new(HashMap::new())),
            active_workspace_id: Arc::new(RwLock::new(None)),
            event_listeners: Arc::new(Mutex::new(Vec::new())),
            config,
            metrics: Arc::new(RwLock::new(WorkspaceMetrics::default())),
            persistence: Arc::new(Mutex::new(persistence)),
        })
    }

    /// Create a new workspace manager with custom persistence config
    pub fn new_with_persistence(
        config: WorkspaceManagerConfig,
        persistence_config: SimplePersistenceConfig,
    ) -> Result<Self> {
        let persistence = SimpleConfigPersistence::new(persistence_config);

        Ok(WorkspaceManager {
            workspaces: Arc::new(RwLock::new(HashMap::new())),
            active_workspace_id: Arc::new(RwLock::new(None)),
            event_listeners: Arc::new(Mutex::new(Vec::new())),
            config,
            metrics: Arc::new(RwLock::new(WorkspaceMetrics::default())),
            persistence: Arc::new(Mutex::new(persistence)),
        })
    }

    /// Initialize the workspace manager and load saved workspaces
    pub async fn initialize(&self) -> Result<()> {
        // Initialize configuration directory
        {
            let persistence = self.persistence.lock().await;
            persistence.initialize_config_directory().map_err(|e| {
                TilleRSError::ConfigurationError(format!(
                    "Failed to initialize config directory: {}",
                    e
                ))
            })?;
        }

        // Load existing workspaces
        self.load_workspaces_from_config().await?;

        // Restore last active workspace if configured
        if self.config.restore_last_active {
            self.restore_last_active_workspace().await?;
        }

        info!("WorkspaceManager initialized successfully");
        Ok(())
    }

    /// Load workspaces from configuration files
    pub async fn load_workspaces_from_config(&self) -> Result<()> {
        let persistence = self.persistence.lock().await;

        match persistence.load_workspaces() {
            Ok(workspaces) => {
                let mut workspace_map = HashMap::new();
                for workspace in workspaces {
                    workspace_map.insert(workspace.id, workspace);
                }

                let workspace_count = workspace_map.len();
                let mut ws_lock = self.workspaces.write().await;
                *ws_lock = workspace_map;
                drop(ws_lock);

                info!("Loaded {} workspaces from configuration", workspace_count);
                Ok(())
            }
            Err(SimplePersistenceError::Io(ref e)) if e.kind() == std::io::ErrorKind::NotFound => {
                // No configuration file exists yet, this is okay for first run
                info!("No existing workspace configuration found, starting fresh");
                Ok(())
            }
            Err(e) => {
                warn!("Failed to load workspaces from config: {}", e);
                Err(
                    TilleRSError::ConfigurationError(format!("Failed to load workspaces: {}", e))
                        .into(),
                )
            }
        }
    }

    /// Save workspaces to configuration files
    pub async fn save_workspaces_to_config(&self) -> Result<()> {
        if !self.config.auto_save {
            return Ok(());
        }

        let workspaces = {
            let ws_lock = self.workspaces.read().await;
            ws_lock.values().cloned().collect::<Vec<_>>()
        };

        let persistence = self.persistence.lock().await;
        persistence.save_workspaces(&workspaces).map_err(|e| {
            TilleRSError::ConfigurationError(format!("Failed to save workspaces: {}", e))
        })?;

        debug!("Saved {} workspaces to configuration", workspaces.len());
        Ok(())
    }

    /// Restore the last active workspace from configuration
    async fn restore_last_active_workspace(&self) -> Result<()> {
        let workspaces = self.workspaces.read().await;

        // Find the workspace with the most recent last_used timestamp
        let last_active = workspaces
            .values()
            .filter(|w| w.last_used.is_some())
            .max_by(|a, b| a.last_used.cmp(&b.last_used));

        if let Some(workspace) = last_active {
            let workspace_id = workspace.id;
            let workspace_name = workspace.name.clone();
            drop(workspaces);

            self.switch_to_workspace(workspace_id).await?;
            info!("Restored last active workspace: {}", workspace_name);
        }

        Ok(())
    }

    /// Create a new workspace
    pub async fn create_workspace(
        &self,
        request: WorkspaceCreateRequest,
        default_pattern_id: Uuid,
    ) -> Result<Uuid> {
        let start_time = std::time::Instant::now();

        // Check workspace limit
        let workspaces = self.workspaces.read().await;
        if workspaces.len() >= self.config.max_workspaces {
            self.increment_error_count().await;
            return Err(TilleRSError::ValidationError(format!(
                "Maximum workspace limit of {} reached",
                self.config.max_workspaces
            ))
            .into());
        }

        // Check for duplicate names
        if workspaces.values().any(|w| w.name == request.name) {
            self.increment_error_count().await;
            return Err(TilleRSError::ValidationError(format!(
                "Workspace name '{}' already exists",
                request.name
            ))
            .into());
        }

        // Check for duplicate shortcuts
        if workspaces
            .values()
            .any(|w| w.keyboard_shortcut == request.keyboard_shortcut)
        {
            self.increment_error_count().await;
            return Err(TilleRSError::ValidationError(format!(
                "Keyboard shortcut '{}' already exists",
                request.keyboard_shortcut
            ))
            .into());
        }
        drop(workspaces);

        // Create workspace
        let workspace = Workspace::new(request, default_pattern_id)?;

        let workspace_id = workspace.id;

        // Store workspace
        let mut workspaces = self.workspaces.write().await;
        workspaces.insert(workspace_id, workspace.clone());
        drop(workspaces);

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.created_count += 1;
        }

        // Emit event
        self.emit_event(WorkspaceEvent::WorkspaceCreated { workspace })
            .await;

        // Set as active if first workspace
        let active_id = self.active_workspace_id.read().await;
        if active_id.is_none() {
            drop(active_id);
            self.switch_to_workspace(workspace_id).await?;
        }

        // Save to configuration
        if let Err(e) = self.save_workspaces_to_config().await {
            warn!(
                "Failed to save workspace configuration after creation: {}",
                e
            );
        }

        let elapsed = start_time.elapsed().as_millis() as f64;
        debug!("Created workspace {} in {}ms", workspace_id, elapsed);

        Ok(workspace_id)
    }

    /// Get a workspace by ID
    pub async fn get_workspace(&self, workspace_id: Uuid) -> Result<Workspace> {
        let workspaces = self.workspaces.read().await;
        workspaces
            .get(&workspace_id)
            .cloned()
            .ok_or_else(|| TilleRSError::WorkspaceNotFound(workspace_id.to_string()).into())
    }

    /// Get all workspaces
    pub async fn get_all_workspaces(&self) -> Vec<Workspace> {
        let workspaces = self.workspaces.read().await;
        workspaces.values().cloned().collect()
    }

    /// Get the currently active workspace
    pub async fn get_active_workspace(&self) -> Option<Workspace> {
        let active_id = self.active_workspace_id.read().await;
        if let Some(id) = *active_id {
            let workspaces = self.workspaces.read().await;
            workspaces.get(&id).cloned()
        } else {
            None
        }
    }

    /// Get the active workspace ID
    pub async fn get_active_workspace_id(&self) -> Option<Uuid> {
        *self.active_workspace_id.read().await
    }

    /// Update a workspace
    pub async fn update_workspace(&self, workspace: Workspace) -> Result<()> {
        let workspace_id = workspace.id;

        // Check for name conflicts (excluding self)
        let workspaces = self.workspaces.read().await;
        if workspaces
            .values()
            .any(|w| w.id != workspace_id && w.name == workspace.name)
        {
            return Err(TilleRSError::ValidationError(format!(
                "Workspace name '{}' already exists",
                workspace.name
            ))
            .into());
        }

        // Check for shortcut conflicts (excluding self)
        if workspaces
            .values()
            .any(|w| w.id != workspace_id && w.keyboard_shortcut == workspace.keyboard_shortcut)
        {
            return Err(TilleRSError::ValidationError(format!(
                "Keyboard shortcut '{}' already exists",
                workspace.keyboard_shortcut
            ))
            .into());
        }
        drop(workspaces);

        // Update workspace
        let mut workspaces = self.workspaces.write().await;
        if let Entry::Occupied(mut existing) = workspaces.entry(workspace_id) {
            existing.insert(workspace.clone());
        } else {
            return Err(TilleRSError::WorkspaceNotFound(workspace_id.to_string()).into());
        }
        drop(workspaces);

        // Emit event
        self.emit_event(WorkspaceEvent::WorkspaceUpdated { workspace })
            .await;

        // Save to configuration
        if let Err(e) = self.save_workspaces_to_config().await {
            warn!("Failed to save workspace configuration after update: {}", e);
        }

        info!("Updated workspace {}", workspace_id);
        Ok(())
    }

    /// Delete a workspace
    pub async fn delete_workspace(&self, workspace_id: Uuid) -> Result<()> {
        // Check if workspace exists
        let mut workspaces = self.workspaces.write().await;
        if !workspaces.contains_key(&workspace_id) {
            return Err(TilleRSError::WorkspaceNotFound(workspace_id.to_string()).into());
        }

        // Remove workspace
        workspaces.remove(&workspace_id);
        let remaining_count = workspaces.len();
        drop(workspaces);

        // Handle active workspace deletion
        let active_id = self.active_workspace_id.read().await;
        if active_id.as_ref() == Some(&workspace_id) {
            drop(active_id);

            // Switch to another workspace if available
            if remaining_count > 0 {
                let workspaces = self.workspaces.read().await;
                if let Some(&first_id) = workspaces.keys().next() {
                    drop(workspaces);
                    self.switch_to_workspace(first_id).await?;
                }
            } else {
                // No workspaces left
                let mut active_id = self.active_workspace_id.write().await;
                *active_id = None;
            }
        }

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.deleted_count += 1;
        }

        // Emit event
        self.emit_event(WorkspaceEvent::WorkspaceDeleted { workspace_id })
            .await;

        // Save to configuration
        if let Err(e) = self.save_workspaces_to_config().await {
            warn!(
                "Failed to save workspace configuration after deletion: {}",
                e
            );
        }

        info!("Deleted workspace {}", workspace_id);
        Ok(())
    }

    /// Switch to a workspace
    pub async fn switch_to_workspace(&self, workspace_id: Uuid) -> Result<()> {
        let start_time = std::time::Instant::now();

        // Check if workspace exists
        let workspaces = self.workspaces.read().await;
        let workspace = workspaces
            .get(&workspace_id)
            .ok_or_else(|| TilleRSError::WorkspaceNotFound(workspace_id.to_string()))?
            .clone();
        drop(workspaces);

        // Get previous active workspace
        let previous_id = {
            let mut active_id = self.active_workspace_id.write().await;
            let previous = *active_id;
            *active_id = Some(workspace_id);
            previous
        };

        // Update last_used timestamp
        {
            let mut workspaces = self.workspaces.write().await;
            if let Some(workspace) = workspaces.get_mut(&workspace_id) {
                workspace.last_used = Some(chrono::Utc::now());
            }
        }

        // Update metrics
        let elapsed = start_time.elapsed().as_millis() as f64;
        {
            let mut metrics = self.metrics.write().await;
            metrics.switch_count += 1;
            metrics.last_switch_time_ms = elapsed;

            // Update running average
            if metrics.switch_count == 1 {
                metrics.avg_switch_time_ms = elapsed;
            } else {
                metrics.avg_switch_time_ms =
                    (metrics.avg_switch_time_ms * (metrics.switch_count - 1) as f64 + elapsed)
                        / metrics.switch_count as f64;
            }
        }

        // Emit event
        self.emit_event(WorkspaceEvent::WorkspaceActivated {
            workspace,
            previous: previous_id,
        })
        .await;

        info!("Switched to workspace {} in {}ms", workspace_id, elapsed);
        Ok(())
    }

    /// Find workspaces by name pattern
    pub async fn find_workspaces_by_name(&self, pattern: &str) -> Vec<Workspace> {
        let workspaces = self.workspaces.read().await;
        workspaces
            .values()
            .filter(|w| w.name.to_lowercase().contains(&pattern.to_lowercase()))
            .cloned()
            .collect()
    }

    /// Get workspaces by keyboard shortcut
    pub async fn find_workspace_by_shortcut(&self, shortcut: &str) -> Option<Workspace> {
        let workspaces = self.workspaces.read().await;
        workspaces
            .values()
            .find(|w| w.keyboard_shortcut == shortcut)
            .cloned()
    }

    /// Get workspace count
    pub async fn get_workspace_count(&self) -> usize {
        let workspaces = self.workspaces.read().await;
        workspaces.len()
    }

    /// List all workspaces currently tracked by the manager
    pub async fn list_workspaces(&self) -> Vec<Workspace> {
        let workspaces = self.workspaces.read().await;
        workspaces.values().cloned().collect()
    }

    /// Check if workspace limit is reached
    pub async fn is_limit_reached(&self) -> bool {
        let count = self.get_workspace_count().await;
        count >= self.config.max_workspaces
    }

    /// Add an event listener
    pub async fn add_event_listener<F>(&self, listener: F)
    where
        F: Fn(WorkspaceEvent) + Send + Sync + 'static,
    {
        let mut listeners = self.event_listeners.lock().await;
        listeners.push(Box::new(listener));
    }

    /// Get performance metrics
    pub async fn get_metrics(&self) -> WorkspaceMetrics {
        let metrics = self.metrics.read().await;
        WorkspaceMetrics {
            switch_count: metrics.switch_count,
            avg_switch_time_ms: metrics.avg_switch_time_ms,
            last_switch_time_ms: metrics.last_switch_time_ms,
            created_count: metrics.created_count,
            deleted_count: metrics.deleted_count,
            error_count: metrics.error_count,
        }
    }

    /// Reset performance metrics
    pub async fn reset_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        *metrics = WorkspaceMetrics::default();
    }

    /// Export configuration to a file (simplified version)
    pub async fn export_config(&self, export_path: &std::path::Path) -> Result<()> {
        let workspaces = {
            let ws_lock = self.workspaces.read().await;
            ws_lock.values().cloned().collect::<Vec<_>>()
        };

        let content = toml::to_string_pretty(&workspaces).map_err(|e| {
            TilleRSError::ConfigurationError(format!("Failed to serialize workspaces: {}", e))
        })?;

        std::fs::write(export_path, content).map_err(|e| {
            TilleRSError::ConfigurationError(format!("Failed to write export file: {}", e))
        })?;

        info!("Exported configuration to {}", export_path.display());
        Ok(())
    }

    /// Validate workspace configuration consistency
    pub async fn validate_all_workspaces(&self) -> Vec<(Uuid, TilleRSError)> {
        let workspaces = self.workspaces.read().await;
        let mut errors = Vec::new();

        // Basic validation - check for duplicate names and shortcuts
        let mut names = HashMap::new();
        let mut shortcuts = HashMap::new();

        for (id, workspace) in workspaces.iter() {
            // Check duplicate names
            if let Some(existing_id) = names.insert(workspace.name.clone(), *id) {
                if existing_id != *id {
                    errors.push((
                        *id,
                        TilleRSError::ValidationError(format!(
                            "Duplicate workspace name: {}",
                            workspace.name
                        )),
                    ));
                }
            }

            // Check duplicate shortcuts
            if let Some(existing_id) = shortcuts.insert(workspace.keyboard_shortcut.clone(), *id) {
                if existing_id != *id {
                    errors.push((
                        *id,
                        TilleRSError::ValidationError(format!(
                            "Duplicate keyboard shortcut: {}",
                            workspace.keyboard_shortcut
                        )),
                    ));
                }
            }
        }

        errors
    }

    /// Internal helper to emit events
    async fn emit_event(&self, event: WorkspaceEvent) {
        let listeners = self.event_listeners.lock().await;
        for listener in listeners.iter() {
            listener(event.clone());
        }
    }

    /// Internal helper to increment error count
    async fn increment_error_count(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.error_count += 1;
    }
}

/// Builder for creating workspace manager with custom configuration
pub struct WorkspaceManagerBuilder {
    config: WorkspaceManagerConfig,
}

impl WorkspaceManagerBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        WorkspaceManagerBuilder {
            config: WorkspaceManagerConfig::default(),
        }
    }

    /// Set maximum number of workspaces
    pub fn max_workspaces(mut self, max: usize) -> Self {
        self.config.max_workspaces = max;
        self
    }

    /// Enable or disable auto-save
    pub fn auto_save(mut self, enabled: bool) -> Self {
        self.config.auto_save = enabled;
        self
    }

    /// Set auto-save interval in seconds
    pub fn auto_save_interval(mut self, seconds: u64) -> Self {
        self.config.auto_save_interval = seconds;
        self
    }

    /// Enable or disable restoring last active workspace
    pub fn restore_last_active(mut self, enabled: bool) -> Self {
        self.config.restore_last_active = enabled;
        self
    }

    /// Enable or disable performance monitoring
    pub fn performance_monitoring(mut self, enabled: bool) -> Self {
        self.config.performance_monitoring = enabled;
        self
    }

    /// Build the workspace manager
    pub fn build(self) -> Result<WorkspaceManager> {
        WorkspaceManager::new(self.config)
    }

    /// Build the workspace manager with custom persistence config
    pub fn build_with_persistence(
        self,
        persistence_config: SimplePersistenceConfig,
    ) -> Result<WorkspaceManager> {
        WorkspaceManager::new_with_persistence(self.config, persistence_config)
    }
}

impl Default for WorkspaceManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_create_workspace() {
        let manager = WorkspaceManager::new(WorkspaceManagerConfig::default());
        let default_pattern_id = Uuid::new_v4();

        let request = WorkspaceCreateRequest {
            name: "Test Workspace".to_string(),
            description: Some("Test description".to_string()),
            keyboard_shortcut: "Cmd+1".to_string(),
            tiling_pattern_id: None,
            auto_arrange: Some(true),
        };

        let workspace_id = manager.create_workspace(request, default_pattern_id).await;

        assert!(workspace_id.is_ok());

        let workspace = manager.get_workspace(workspace_id.unwrap()).await;
        assert!(workspace.is_ok());
        assert_eq!(workspace.unwrap().name, "Test Workspace");
    }

    #[tokio::test]
    async fn test_duplicate_workspace_name() {
        let manager = WorkspaceManager::new(WorkspaceManagerConfig::default());
        let default_pattern_id = Uuid::new_v4();

        // Create first workspace
        let request1 = WorkspaceCreateRequest {
            name: "Duplicate".to_string(),
            description: None,
            keyboard_shortcut: "Cmd+1".to_string(),
            tiling_pattern_id: None,
            auto_arrange: None,
        };
        let result1 = manager.create_workspace(request1, default_pattern_id).await;
        assert!(result1.is_ok());

        // Try to create duplicate
        let request2 = WorkspaceCreateRequest {
            name: "Duplicate".to_string(),
            description: None,
            keyboard_shortcut: "Cmd+2".to_string(),
            tiling_pattern_id: None,
            auto_arrange: None,
        };
        let result2 = manager.create_workspace(request2, default_pattern_id).await;
        assert!(result2.is_err());
    }

    #[tokio::test]
    async fn test_workspace_switching() {
        let manager = WorkspaceManager::new(WorkspaceManagerConfig::default());

        // Create workspaces
        let id1 = manager.create_workspace("WS1".to_string(), None, None, None).await.unwrap();
        let id2 = manager.create_workspace("WS2".to_string(), None, None, None).await.unwrap();

        // Switch to second workspace
        assert!(manager.switch_to_workspace(id2).await.is_ok());

        // Verify active workspace
        let active = manager.get_active_workspace_id().await;
        assert_eq!(active, Some(id2));

        // Switch back to first
        assert!(manager.switch_to_workspace(id1).await.is_ok());
        let active = manager.get_active_workspace_id().await;
        assert_eq!(active, Some(id1));
    }

    #[tokio::test]
    async fn test_workspace_deletion() {
        let manager = WorkspaceManager::new(WorkspaceManagerConfig::default());

        let id1 = manager.create_workspace("WS1".to_string(), None, None, None).await.unwrap();
        let id2 = manager.create_workspace("WS2".to_string(), None, None, None).await.unwrap();

        // Delete first workspace
        assert!(manager.delete_workspace(id1).await.is_ok());

        // Verify it's gone
        assert!(manager.get_workspace(id1).await.is_err());

        // Verify count
        assert_eq!(manager.get_workspace_count().await, 1);
    }

    #[tokio::test]
    async fn test_workspace_search() {
        let manager = WorkspaceManager::new(WorkspaceManagerConfig::default());

        manager.create_workspace("Development".to_string(), None, None, None).await.unwrap();
        manager.create_workspace("Testing".to_string(), None, None, None).await.unwrap();
        manager.create_workspace("Production".to_string(), None, None, None).await.unwrap();

        let results = manager.find_workspaces_by_name("dev").await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Development");

        let results = manager.find_workspaces_by_name("tion").await;
        assert_eq!(results.len(), 2); // Testing and Production
    }

    #[tokio::test]
    async fn test_workspace_limit() {
        let config = WorkspaceManagerConfig {
            max_workspaces: 2,
            ..Default::default()
        };
        let manager = WorkspaceManager::new(config);

        // Create maximum allowed workspaces
        assert!(manager.create_workspace("WS1".to_string(), None, None, None).await.is_ok());
        assert!(manager.create_workspace("WS2".to_string(), None, None, None).await.is_ok());

        // Try to create one more
        let result = manager.create_workspace("WS3".to_string(), None, None, None).await;
        assert!(result.is_err());
        assert!(manager.is_limit_reached().await);
    }

    #[tokio::test]
    async fn test_performance_metrics() {
        let manager = WorkspaceManager::new(WorkspaceManagerConfig::default());

        let id1 = manager.create_workspace("WS1".to_string(), None, None, None).await.unwrap();
        let id2 = manager.create_workspace("WS2".to_string(), None, None, None).await.unwrap();

        // Perform some switches
        manager.switch_to_workspace(id2).await.unwrap();
        manager.switch_to_workspace(id1).await.unwrap();

        let metrics = manager.get_metrics().await;
        assert_eq!(metrics.switch_count, 3); // 1 auto-switch + 2 manual
        assert_eq!(metrics.created_count, 2);
        assert!(metrics.avg_switch_time_ms > 0.0);
    }
}
*/
