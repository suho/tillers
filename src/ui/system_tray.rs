//! System tray integration for TilleRS (stub implementation)
//!
//! Provides a lightweight, cross-platform placeholder that tracks
//! health status without invoking macOS-specific APIs. This keeps the
//! architecture wired together while the concrete UI layer evolves.

use crate::{
    error_recovery::ErrorRecoveryManager,
    permissions::PermissionChecker,
    services::{WindowManager, WorkspaceManager},
    Result,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Display preferences for the (future) system tray UI
#[derive(Debug, Clone)]
pub struct SystemTrayConfig {
    /// Whether the tray icon should be shown at all
    pub show_icon: bool,
    /// Include workspace counts in status summaries
    pub show_workspace_count: bool,
    /// Show detailed status indicators in the menu
    pub show_status_indicators: bool,
    /// Automatically hide the tray when everything is healthy
    pub auto_hide_when_healthy: bool,
    /// Polling interval used by the lifecycle manager (seconds)
    pub update_interval_secs: u64,
}

impl Default for SystemTrayConfig {
    fn default() -> Self {
        Self {
            show_icon: true,
            show_workspace_count: true,
            show_status_indicators: true,
            auto_hide_when_healthy: false,
            update_interval_secs: 5,
        }
    }
}

/// High-level status exposed to the (future) UI layer
#[derive(Debug, Clone, PartialEq)]
pub enum TrayStatus {
    Healthy,
    PermissionsRequired,
    Degraded,
    Error,
}

impl TrayStatus {
    pub fn description(&self) -> &'static str {
        match self {
            TrayStatus::Healthy => "TilleRS is running normally",
            TrayStatus::PermissionsRequired => "Additional permissions required",
            TrayStatus::Degraded => "Running with limited functionality",
            TrayStatus::Error => "Critical errors detected",
        }
    }
}

/// Minimal manager that keeps track of status and exposes hooks used by
/// the lifecycle manager. macOS integration will be layered on later.
pub struct SystemTrayManager {
    config: SystemTrayConfig,
    workspace_manager: Arc<WorkspaceManager>,
    _window_manager: Arc<WindowManager>,
    error_recovery: Arc<ErrorRecoveryManager>,
    permission_checker: Arc<RwLock<PermissionChecker>>,
    current_status: TrayStatus,
    visible: bool,
}

impl SystemTrayManager {
    pub fn new(
        config: SystemTrayConfig,
        workspace_manager: Arc<WorkspaceManager>,
        window_manager: Arc<WindowManager>,
        error_recovery: Arc<ErrorRecoveryManager>,
        permission_checker: Arc<RwLock<PermissionChecker>>,
    ) -> Self {
        Self {
            config,
            workspace_manager,
            _window_manager: window_manager,
            error_recovery,
            permission_checker,
            current_status: TrayStatus::Healthy,
            visible: true,
        }
    }

    /// Placeholder initialise hook – determines the starting status and
    /// logs the intent. Future macOS-specific wiring can slot in here.
    pub async fn initialize(&mut self) -> Result<()> {
        debug!("Initializing system tray stub");
        self.current_status = self.determine_current_status().await?;
        self.visible = self.config.show_icon;
        info!(
            "System tray stub initialised ({})",
            self.current_status.description()
        );
        Ok(())
    }

    /// Recalculate status based on permission and recovery state.
    pub async fn update_status(&mut self) -> Result<()> {
        let new_status = self.determine_current_status().await?;
        if new_status != self.current_status {
            info!(
                "System tray status changed: {:?} -> {:?}",
                self.current_status, new_status
            );
            self.current_status = new_status;
        }
        Ok(())
    }

    /// Apply visibility rules. The stub simply flips an internal flag so
    /// lifecycle tests can observe behaviour without UI side-effects.
    pub async fn update_visibility(&mut self) -> Result<()> {
        if !self.config.auto_hide_when_healthy {
            return Ok(());
        }

        self.visible = self.current_status != TrayStatus::Healthy;
        Ok(())
    }

    pub fn current_status(&self) -> &TrayStatus {
        &self.current_status
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    async fn determine_current_status(&self) -> Result<TrayStatus> {
        let permissions_granted = {
            let mut checker = self.permission_checker.write().await;
            checker.all_required_permissions_granted().await?
        };

        if !permissions_granted {
            return Ok(TrayStatus::PermissionsRequired);
        }

        let health = self.error_recovery.get_health_status().await?;
        if !health.is_healthy() {
            if health.active_circuit_breakers.is_empty() {
                return Ok(TrayStatus::Degraded);
            }
            return Ok(TrayStatus::Error);
        }

        // Light-touch visibility hint for workspace count (logged only)
        if self.config.show_workspace_count {
            let count = self.workspace_manager.get_workspace_count().await;
            debug!("Workspace count for tray summary: {}", count);
        }

        Ok(TrayStatus::Healthy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::PermissionConfig;
    use crate::services::workspace_manager::WorkspaceManagerConfig;

    fn seed_managers() -> (
        Arc<WorkspaceManager>,
        Arc<WindowManager>,
        Arc<ErrorRecoveryManager>,
        Arc<RwLock<PermissionChecker>>,
    ) {
        let workspace_manager = Arc::new(
            WorkspaceManager::new(WorkspaceManagerConfig::default()).expect("workspace manager"),
        );
        let accessibility: Arc<dyn crate::macos::accessibility::AccessibilityProvider> =
            Arc::new(crate::macos::accessibility::InMemoryAccessibilityProvider::default());
        let displays: Arc<dyn crate::macos::core_graphics::DisplayProvider> =
            Arc::new(crate::macos::core_graphics::InMemoryDisplayProvider::default());
        let window_manager = Arc::new(WindowManager::new(accessibility, displays));
        let permission_checker = Arc::new(RwLock::new(PermissionChecker::new(
            PermissionConfig::default(),
        )));
        let recovery = Arc::new(ErrorRecoveryManager::new(
            crate::error_recovery::RecoveryConfig::default(),
            PermissionChecker::new(PermissionConfig::default()),
        ));
        (
            workspace_manager,
            window_manager,
            recovery,
            permission_checker,
        )
    }

    #[tokio::test]
    async fn initialise_stub_sets_status() {
        let (workspace_manager, window_manager, recovery, permissions) = seed_managers();
        let mut manager = SystemTrayManager::new(
            SystemTrayConfig::default(),
            workspace_manager,
            window_manager,
            recovery,
            permissions,
        );

        manager.initialize().await.unwrap();
        assert_eq!(manager.current_status(), &TrayStatus::Healthy);
        assert!(manager.is_visible());
    }
}
