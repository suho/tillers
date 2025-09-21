use crate::models::keyboard_mapping::ActionParameters;
use crate::models::TilingPattern;
use crate::services::{
    keyboard_handler::{KeyboardEvent, KeyboardHandler},
    tiling_engine::{TilingEngine, WindowLayout},
    window_manager::{WindowInfo, WindowManager, WindowMode},
    workspace_manager::{WorkspaceEvent, WorkspaceManager},
};
use crate::{Result, TilleRSError};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Configuration for the workspace orchestrator
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Whether to automatically arrange windows when switching workspaces
    pub auto_arrange_on_switch: bool,
    /// Whether to automatically arrange windows when new windows are detected
    pub auto_arrange_on_new_window: bool,
    /// Delay in milliseconds before applying layout (allows windows to settle)
    pub layout_delay_ms: u64,
    /// Maximum number of windows to tile automatically
    pub max_auto_tile_windows: usize,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            auto_arrange_on_switch: true,
            auto_arrange_on_new_window: true,
            layout_delay_ms: 100,
            max_auto_tile_windows: 20,
        }
    }
}

/// Orchestrates the interaction between workspace management, window management, and tiling
pub struct WorkspaceOrchestrator {
    workspace_manager: Arc<WorkspaceManager>,
    window_manager: Arc<WindowManager>,
    tiling_engine: Arc<TilingEngine>,
    keyboard_handler: Arc<KeyboardHandler>,
    patterns: Arc<RwLock<HashMap<Uuid, TilingPattern>>>,
    config: OrchestratorConfig,
}

impl WorkspaceOrchestrator {
    pub fn new(
        workspace_manager: Arc<WorkspaceManager>,
        window_manager: Arc<WindowManager>,
        tiling_engine: Arc<TilingEngine>,
        keyboard_handler: Arc<KeyboardHandler>,
        config: OrchestratorConfig,
    ) -> Self {
        Self {
            workspace_manager,
            window_manager,
            tiling_engine,
            keyboard_handler,
            patterns: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Initialize the orchestrator and set up event listeners
    pub async fn initialize(&self) -> Result<()> {
        // Initialize the workspace manager
        self.workspace_manager.initialize().await?;

        // Set up event listeners for workspace changes
        self.setup_workspace_event_listeners().await;

        info!("WorkspaceOrchestrator initialized successfully");
        Ok(())
    }

    /// Add a tiling pattern to the orchestrator's pattern registry
    pub async fn add_tiling_pattern(&self, pattern: TilingPattern) {
        let mut patterns = self.patterns.write().await;
        patterns.insert(pattern.id, pattern);
    }

    /// Apply automatic layout to the current workspace
    pub async fn arrange_current_workspace(&self) -> Result<()> {
        // Get the active workspace
        let active_workspace = self.workspace_manager.get_active_workspace().await;
        let workspace = match active_workspace {
            Some(ws) => ws,
            None => {
                debug!("No active workspace to arrange");
                return Ok(());
            }
        };

        if !workspace.auto_arrange {
            debug!("Auto-arrange disabled for workspace '{}'", workspace.name);
            return Ok(());
        }

        // Get the tiling pattern for this workspace
        let patterns = self.patterns.read().await;
        let pattern = match patterns.get(&workspace.tiling_pattern_id) {
            Some(p) => p.clone(),
            None => {
                warn!(
                    "Tiling pattern {} not found for workspace '{}'",
                    workspace.tiling_pattern_id, workspace.name
                );
                return Err(TilleRSError::ConfigurationError(format!(
                    "Tiling pattern {} not found",
                    workspace.tiling_pattern_id
                ))
                .into());
            }
        };
        drop(patterns);

        // Get windows to tile
        let windows = self.get_tileable_windows().await?;
        if windows.is_empty() {
            debug!(
                "No tileable windows found for workspace '{}'",
                workspace.name
            );
            return Ok(());
        }

        if windows.len() > self.config.max_auto_tile_windows {
            warn!(
                "Too many windows ({}) for auto-tiling, limit is {}",
                windows.len(),
                self.config.max_auto_tile_windows
            );
            return Ok(());
        }

        // Get available area for tiling (primary monitor for now)
        let available_area = self.get_primary_tiling_area().await?;

        // Calculate window layouts
        let window_ids: Vec<u32> = windows.iter().map(|w| w.id).collect();
        let layouts = self
            .tiling_engine
            .layout_windows(&window_ids, &pattern, available_area)
            .await?;

        // Apply the layouts with a small delay to allow windows to settle
        if self.config.layout_delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(
                self.config.layout_delay_ms,
            ))
            .await;
        }

        self.apply_window_layouts(&layouts).await?;

        info!(
            "Applied tiling layout to {} windows in workspace '{}'",
            layouts.len(),
            workspace.name
        );

        Ok(())
    }

    /// Get windows that are suitable for tiling
    async fn get_tileable_windows(&self) -> Result<Vec<WindowInfo>> {
        let all_windows = self.window_manager.list_windows(true, None).await?;

        let tileable_windows: Vec<WindowInfo> = all_windows
            .into_iter()
            .filter(|window| {
                // Only tile windows that are:
                // - Not minimized
                // - Not fullscreen
                // - In tiled mode (not floating)
                // - Have a reasonable size
                !window.is_minimized
                    && !window.is_fullscreen
                    && window.mode == WindowMode::Tiled
                    && window.size.width > 100.0
                    && window.size.height > 100.0
            })
            .collect();

        Ok(tileable_windows)
    }

    /// Get the primary monitor's tiling area
    async fn get_primary_tiling_area(&self) -> Result<crate::macos::accessibility::Rect> {
        // For now, use a default area. In a full implementation, this would:
        // 1. Get the primary monitor bounds
        // 2. Subtract menu bar, dock, and other system UI elements
        // 3. Apply any workspace-specific margins

        use crate::macos::accessibility::{Point, Rect, Size};

        // Default to a reasonable screen area (this should be replaced with actual monitor detection)
        Ok(Rect {
            origin: Point { x: 0.0, y: 25.0 }, // Account for menu bar
            size: Size {
                width: 1920.0,
                height: 1055.0,
            }, // 1080p minus menu bar and dock
        })
    }

    /// Apply the calculated window layouts
    async fn apply_window_layouts(&self, layouts: &[WindowLayout]) -> Result<()> {
        for layout in layouts {
            if let Err(e) = self
                .window_manager
                .set_window_frame(layout.window_id, layout.frame, true)
                .await
            {
                warn!("Failed to set frame for window {}: {}", layout.window_id, e);
                // Continue with other windows rather than failing completely
            }
        }
        Ok(())
    }

    /// Set up event listeners for workspace changes
    async fn setup_workspace_event_listeners(&self) {
        let orchestrator_clone = Arc::new(WorkspaceOrchestrator {
            workspace_manager: self.workspace_manager.clone(),
            window_manager: self.window_manager.clone(),
            tiling_engine: self.tiling_engine.clone(),
            keyboard_handler: self.keyboard_handler.clone(),
            patterns: self.patterns.clone(),
            config: self.config.clone(),
        });

        self.workspace_manager
            .add_event_listener(move |event| {
                let orchestrator = orchestrator_clone.clone();
                tokio::spawn(async move {
                    if let Err(e) = orchestrator.handle_workspace_event(event).await {
                        warn!("Failed to handle workspace event: {}", e);
                    }
                });
            })
            .await;
    }

    /// Handle workspace events and trigger appropriate actions
    async fn handle_workspace_event(&self, event: WorkspaceEvent) -> Result<()> {
        match event {
            WorkspaceEvent::WorkspaceActivated { workspace, .. } => {
                if self.config.auto_arrange_on_switch && workspace.auto_arrange {
                    debug!(
                        "Auto-arranging windows for activated workspace '{}'",
                        workspace.name
                    );
                    self.arrange_current_workspace().await?;
                }
            }
            WorkspaceEvent::WorkspaceCreated { .. }
            | WorkspaceEvent::WorkspaceUpdated { .. }
            | WorkspaceEvent::WorkspaceDeleted { .. }
            | WorkspaceEvent::ConfigurationChanged { .. } => {
                // These events might need different handling in the future
                debug!("Received workspace event: {:?}", event);
            }
        }
        Ok(())
    }

    /// Handle keyboard events and dispatch them to appropriate actions
    pub async fn handle_keyboard_event(&self, event: KeyboardEvent) -> Result<()> {
        use crate::models::keyboard_mapping::ActionType;

        debug!("Handling keyboard event: {:?}", event);

        match event.action {
            ActionType::SwitchWorkspace => {
                if let ActionParameters::WorkspaceId(workspace_id) = event.parameters {
                    self.workspace_manager
                        .switch_to_workspace(workspace_id)
                        .await?;
                } else {
                    // Try to find workspace by position or other criteria
                    self.handle_workspace_switch_by_position().await?;
                }
            }
            ActionType::MoveWindow => {
                if let ActionParameters::WorkspaceId(target_workspace_id) = event.parameters {
                    self.move_focused_window_to_workspace(target_workspace_id)
                        .await?;
                }
            }
            ActionType::MoveWindowToMonitor => {
                if let ActionParameters::MonitorId(monitor_id) = event.parameters {
                    self.move_focused_window_to_monitor(&monitor_id).await?;
                }
            }
            ActionType::CreateWorkspace => {
                if let ActionParameters::WorkspaceName(name) = event.parameters {
                    self.create_new_workspace(&name).await?;
                }
            }
            ActionType::DeleteWorkspace => {
                self.delete_current_workspace().await?;
            }
            ActionType::ToggleFloating => {
                self.toggle_focused_window_floating().await?;
            }
            ActionType::ToggleFullscreen => {
                self.toggle_focused_window_fullscreen().await?;
            }
            ActionType::FocusNext => {
                self.focus_next_window().await?;
            }
            ActionType::FocusPrevious => {
                self.focus_previous_window().await?;
            }
            ActionType::CloseWindow => {
                self.close_focused_window().await?;
            }
            ActionType::RefreshLayout => {
                self.arrange_current_workspace().await?;
            }
            ActionType::ShowOverview => {
                // This would show a workspace overview UI
                info!("Workspace overview requested");
            }
            ActionType::Custom(action) => {
                warn!("Custom action '{}' not implemented", action);
            }
            _ => {
                warn!("Unhandled keyboard action: {:?}", event.action);
            }
        }

        Ok(())
    }

    /// Handle workspace switching when no specific workspace ID is provided
    async fn handle_workspace_switch_by_position(&self) -> Result<()> {
        // This could cycle through workspaces or switch to a default
        let workspaces = self.workspace_manager.get_all_workspaces().await;
        if let Some(first_workspace) = workspaces.first() {
            self.workspace_manager
                .switch_to_workspace(first_workspace.id)
                .await?;
        }
        Ok(())
    }

    /// Move the currently focused window to another workspace
    async fn move_focused_window_to_workspace(&self, target_workspace_id: Uuid) -> Result<()> {
        // Get the currently focused window
        let windows = self.window_manager.list_windows(true, None).await?;
        let focused_window = windows.iter().find(|w| w.is_focused);

        if let Some(window) = focused_window {
            info!(
                "Moving window '{}' to workspace {}",
                window.title, target_workspace_id
            );
            // In a full implementation, this would involve:
            // 1. Removing the window from current workspace tracking
            // 2. Adding it to the target workspace
            // 3. Possibly moving it to the target workspace's monitor
            // For now, we'll just log the action
        } else {
            debug!("No focused window found to move");
        }

        Ok(())
    }

    /// Move the currently focused window to a different monitor
    async fn move_focused_window_to_monitor(&self, _monitor_id: &str) -> Result<()> {
        // In a full implementation, this would move the window to the specified monitor
        info!("Move window to monitor requested (not fully implemented)");
        Ok(())
    }

    /// Create a new workspace with the given name
    async fn create_new_workspace(&self, name: &str) -> Result<()> {
        use crate::models::WorkspaceCreateRequest;

        // Create a unique keyboard shortcut (simplified)
        let shortcut = format!("opt+{}", name.chars().next().unwrap_or('x'));

        let request = WorkspaceCreateRequest {
            name: name.to_string(),
            description: Some("Workspace created via keyboard shortcut".to_string()),
            keyboard_shortcut: shortcut,
            tiling_pattern_id: None, // Use default
            auto_arrange: Some(true),
        };

        // Get a default pattern ID (simplified - in real implementation would come from config)
        let patterns = self.patterns.read().await;
        let default_pattern_id = patterns.keys().next().copied().unwrap_or_else(Uuid::new_v4);
        drop(patterns);

        let workspace_id = self
            .workspace_manager
            .create_workspace(request, default_pattern_id)
            .await?;
        info!("Created new workspace '{}' with ID {}", name, workspace_id);

        Ok(())
    }

    /// Delete the current workspace
    async fn delete_current_workspace(&self) -> Result<()> {
        if let Some(active_workspace) = self.workspace_manager.get_active_workspace().await {
            self.workspace_manager
                .delete_workspace(active_workspace.id)
                .await?;
            info!("Deleted workspace '{}'", active_workspace.name);
        }
        Ok(())
    }

    /// Toggle floating mode for the focused window
    async fn toggle_focused_window_floating(&self) -> Result<()> {
        let windows = self.window_manager.list_windows(true, None).await?;
        if let Some(focused_window) = windows.iter().find(|w| w.is_focused) {
            self.window_manager
                .toggle_floating(focused_window.id)
                .await?;
            info!(
                "Toggled floating mode for window '{}'",
                focused_window.title
            );
        }
        Ok(())
    }

    /// Toggle fullscreen mode for the focused window
    async fn toggle_focused_window_fullscreen(&self) -> Result<()> {
        // In a full implementation, this would toggle the window's fullscreen state
        info!("Toggle fullscreen requested (not fully implemented)");
        Ok(())
    }

    /// Focus the next window in the current workspace
    async fn focus_next_window(&self) -> Result<()> {
        let windows = self.get_tileable_windows().await?;
        if windows.len() <= 1 {
            return Ok(());
        }

        // Find currently focused window and focus the next one
        if let Some(current_index) = windows.iter().position(|w| w.is_focused) {
            let next_index = (current_index + 1) % windows.len();
            let next_window = &windows[next_index];
            self.window_manager.focus_window(next_window.id).await?;
            info!("Focused next window: '{}'", next_window.title);
        } else if let Some(first_window) = windows.first() {
            // No focused window, focus the first one
            self.window_manager.focus_window(first_window.id).await?;
            info!("Focused first window: '{}'", first_window.title);
        }

        Ok(())
    }

    /// Focus the previous window in the current workspace
    async fn focus_previous_window(&self) -> Result<()> {
        let windows = self.get_tileable_windows().await?;
        if windows.len() <= 1 {
            return Ok(());
        }

        // Find currently focused window and focus the previous one
        if let Some(current_index) = windows.iter().position(|w| w.is_focused) {
            let prev_index = if current_index == 0 {
                windows.len() - 1
            } else {
                current_index - 1
            };
            let prev_window = &windows[prev_index];
            self.window_manager.focus_window(prev_window.id).await?;
            info!("Focused previous window: '{}'", prev_window.title);
        } else if let Some(last_window) = windows.last() {
            // No focused window, focus the last one
            self.window_manager.focus_window(last_window.id).await?;
            info!("Focused last window: '{}'", last_window.title);
        }

        Ok(())
    }

    /// Close the currently focused window
    async fn close_focused_window(&self) -> Result<()> {
        // In a full implementation, this would send a close command to the focused window
        info!("Close focused window requested (not fully implemented)");
        Ok(())
    }

    /// Manually trigger arrangement of the current workspace
    pub async fn manual_arrange(&self) -> Result<()> {
        info!("Manual workspace arrangement triggered");
        self.arrange_current_workspace().await
    }

    /// Get orchestrator metrics and statistics
    pub async fn get_metrics(&self) -> OrchestratorMetrics {
        let workspace_metrics = self.workspace_manager.get_metrics().await;

        OrchestratorMetrics {
            workspace_switches: workspace_metrics.switch_count,
            windows_tiled: 0, // Would need to track this
            layout_errors: 0, // Would need to track this
        }
    }
}

/// Metrics for the workspace orchestrator
#[derive(Debug, Clone)]
pub struct OrchestratorMetrics {
    pub workspace_switches: u64,
    pub windows_tiled: u64,
    pub layout_errors: u64,
}

/// Builder for creating a workspace orchestrator
pub struct WorkspaceOrchestratorBuilder {
    workspace_manager: Option<Arc<WorkspaceManager>>,
    window_manager: Option<Arc<WindowManager>>,
    tiling_engine: Option<Arc<TilingEngine>>,
    keyboard_handler: Option<Arc<KeyboardHandler>>,
    config: OrchestratorConfig,
}

impl WorkspaceOrchestratorBuilder {
    pub fn new() -> Self {
        Self {
            workspace_manager: None,
            window_manager: None,
            tiling_engine: None,
            keyboard_handler: None,
            config: OrchestratorConfig::default(),
        }
    }

    pub fn workspace_manager(mut self, manager: Arc<WorkspaceManager>) -> Self {
        self.workspace_manager = Some(manager);
        self
    }

    pub fn window_manager(mut self, manager: Arc<WindowManager>) -> Self {
        self.window_manager = Some(manager);
        self
    }

    pub fn tiling_engine(mut self, engine: Arc<TilingEngine>) -> Self {
        self.tiling_engine = Some(engine);
        self
    }

    pub fn keyboard_handler(mut self, handler: Arc<KeyboardHandler>) -> Self {
        self.keyboard_handler = Some(handler);
        self
    }

    pub fn config(mut self, config: OrchestratorConfig) -> Self {
        self.config = config;
        self
    }

    pub fn build(self) -> Result<WorkspaceOrchestrator> {
        let workspace_manager = self.workspace_manager.ok_or_else(|| {
            TilleRSError::ConfigurationError("WorkspaceManager is required".to_string())
        })?;
        let window_manager = self.window_manager.ok_or_else(|| {
            TilleRSError::ConfigurationError("WindowManager is required".to_string())
        })?;
        let tiling_engine = self.tiling_engine.ok_or_else(|| {
            TilleRSError::ConfigurationError("TilingEngine is required".to_string())
        })?;
        let keyboard_handler = self.keyboard_handler.ok_or_else(|| {
            TilleRSError::ConfigurationError("KeyboardHandler is required".to_string())
        })?;

        Ok(WorkspaceOrchestrator::new(
            workspace_manager,
            window_manager,
            tiling_engine,
            keyboard_handler,
            self.config,
        ))
    }
}

impl Default for WorkspaceOrchestratorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tiling_pattern::{LayoutAlgorithm, ResizeBehavior};
    use crate::services::workspace_manager::WorkspaceManagerConfig;

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let workspace_manager = Arc::new(
            WorkspaceManager::new(WorkspaceManagerConfig::default())
                .expect("Failed to create workspace manager"),
        );
        let window_manager = Arc::new(WindowManager::with_default_providers());
        let tiling_engine = Arc::new(TilingEngine::new());
        let keyboard_handler = Arc::new(KeyboardHandler::new(std::collections::HashSet::new()));

        let orchestrator = WorkspaceOrchestratorBuilder::new()
            .workspace_manager(workspace_manager)
            .window_manager(window_manager)
            .tiling_engine(tiling_engine)
            .keyboard_handler(keyboard_handler)
            .build();

        assert!(orchestrator.is_ok());
    }

    #[tokio::test]
    async fn test_orchestrator_with_patterns() {
        let workspace_manager = Arc::new(
            WorkspaceManager::new(WorkspaceManagerConfig::default())
                .expect("Failed to create workspace manager"),
        );
        let window_manager = Arc::new(WindowManager::with_default_providers());
        let tiling_engine = Arc::new(TilingEngine::new());
        let keyboard_handler = Arc::new(KeyboardHandler::new(std::collections::HashSet::new()));

        let orchestrator = WorkspaceOrchestratorBuilder::new()
            .workspace_manager(workspace_manager)
            .window_manager(window_manager)
            .tiling_engine(tiling_engine)
            .keyboard_handler(keyboard_handler)
            .build()
            .expect("Failed to create orchestrator");

        let pattern = TilingPattern {
            id: Uuid::new_v4(),
            name: "Test Pattern".to_string(),
            layout_algorithm: LayoutAlgorithm::MasterStack,
            main_area_ratio: 0.6,
            gap_size: 10,
            window_margin: 5,
            max_windows: 10,
            resize_behavior: ResizeBehavior::Shrink,
        };

        orchestrator.add_tiling_pattern(pattern.clone()).await;

        let patterns = orchestrator.patterns.read().await;
        assert!(patterns.contains_key(&pattern.id));
    }
}
