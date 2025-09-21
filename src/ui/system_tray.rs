//! System tray integration for TilleRS
//!
//! Provides a native macOS system tray icon with status information,
//! basic controls, and quick access to TilleRS functionality.

use crate::{
    error_recovery::{ErrorRecoveryManager, HealthStatus},
    permissions::PermissionChecker,
    services::{WorkspaceManager, WindowManager},
    Result, TilleRSError,
};
use cocoa::{
    appkit::{
        NSApplication, NSApplicationActivationPolicy, NSEventTrackingRunLoopMode,
        NSMenu, NSMenuItem, NSStatusBar, NSStatusItem, NSWindow,
    },
    base::{id, nil, YES, NO},
    foundation::{NSAutoreleasePool, NSString, NSRect, NSPoint, NSSize},
};
use objc::{class, msg_send, sel, sel_impl};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

/// System tray configuration
#[derive(Debug, Clone)]
pub struct SystemTrayConfig {
    /// Whether to show the tray icon
    pub show_icon: bool,
    /// Whether to show workspace count in the icon
    pub show_workspace_count: bool,
    /// Whether to show status indicators
    pub show_status_indicators: bool,
    /// Auto-hide when all permissions are granted
    pub auto_hide_when_healthy: bool,
    /// Update interval for status information
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

/// System tray status
#[derive(Debug, Clone, PartialEq)]
pub enum TrayStatus {
    /// All systems operational
    Healthy,
    /// Missing permissions
    PermissionsRequired,
    /// System errors or circuit breakers active
    Degraded,
    /// Critical errors
    Error,
}

impl TrayStatus {
    /// Get the appropriate icon character for this status
    pub fn icon_char(&self) -> &'static str {
        match self {
            TrayStatus::Healthy => "✓",
            TrayStatus::PermissionsRequired => "⚠",
            TrayStatus::Degraded => "⚡",
            TrayStatus::Error => "✗",
        }
    }

    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            TrayStatus::Healthy => "TilleRS is running normally",
            TrayStatus::PermissionsRequired => "Permissions required",
            TrayStatus::Degraded => "Running with limited functionality",
            TrayStatus::Error => "TilleRS has encountered errors",
        }
    }
}

/// System tray manager for TilleRS
pub struct SystemTrayManager {
    config: SystemTrayConfig,
    workspace_manager: Arc<WorkspaceManager>,
    window_manager: Arc<WindowManager>,
    error_recovery: Arc<ErrorRecoveryManager>,
    permission_checker: Arc<RwLock<PermissionChecker>>,
    
    // macOS-specific fields
    status_item: Option<id>,
    menu: Option<id>,
    current_status: TrayStatus,
}

impl SystemTrayManager {
    /// Create a new system tray manager
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
            window_manager,
            error_recovery,
            permission_checker,
            status_item: None,
            menu: None,
            current_status: TrayStatus::Healthy,
        }
    }

    /// Initialize the system tray
    pub async fn initialize(&mut self) -> Result<()> {
        if !self.config.show_icon {
            debug!("System tray disabled in configuration");
            return Ok();
        }

        info!("Initializing system tray...");

        unsafe {
            let _pool = NSAutoreleasePool::new(nil);

            // Get the shared status bar
            let status_bar = NSStatusBar::systemStatusBar(nil);
            
            // Create a status item
            let status_item = status_bar.statusItemWithLength_(-1.0); // NSVariableStatusItemLength
            
            if status_item == nil {
                return Err(TilleRSError::MacOSAPIError(
                    "Failed to create status bar item".to_string()
                ).into());
            }

            // Set initial title
            self.update_status_item_title(status_item).await;

            // Create menu
            let menu = self.create_menu().await?;
            let _: () = msg_send![status_item, setMenu: menu];

            // Store references
            self.status_item = Some(status_item);
            self.menu = Some(menu);
        }

        info!("System tray initialized successfully");
        Ok(())
    }

    /// Update the system tray status
    pub async fn update_status(&mut self) -> Result<()> {
        if self.status_item.is_none() {
            return Ok(());
        }

        // Determine current status
        let new_status = self.determine_current_status().await?;
        
        if new_status != self.current_status {
            info!("System tray status changed: {:?} -> {:?}", self.current_status, new_status);
            self.current_status = new_status;
            
            // Update the status item
            if let Some(status_item) = self.status_item {
                self.update_status_item_title(status_item).await;
            }
            
            // Update the menu
            if let Some(menu) = self.menu {
                self.update_menu_items(menu).await?;
            }
        }

        Ok(())
    }

    /// Show/hide the system tray based on current health status
    pub async fn update_visibility(&mut self) -> Result<()> {
        if !self.config.auto_hide_when_healthy {
            return Ok();
        }

        let should_hide = self.current_status == TrayStatus::Healthy;
        
        if let Some(status_item) = self.status_item {
            unsafe {
                let _: () = msg_send![status_item, setVisible: if should_hide { NO } else { YES }];
            }
        }

        Ok(())
    }

    /// Cleanup system tray resources
    pub async fn cleanup(&mut self) -> Result<()> {
        info!("Cleaning up system tray...");

        if let Some(status_item) = self.status_item.take() {
            unsafe {
                let status_bar = NSStatusBar::systemStatusBar(nil);
                let _: () = msg_send![status_bar, removeStatusItem: status_item];
            }
        }

        self.menu = None;
        debug!("System tray cleanup complete");
        Ok(())
    }

    // Private helper methods

    async fn determine_current_status(&self) -> Result<TrayStatus> {
        // Check health status from error recovery manager
        let health_status = self.error_recovery.get_health_status().await?;
        
        if !health_status.permissions_granted {
            return Ok(TrayStatus::PermissionsRequired);
        }
        
        if !health_status.active_circuit_breakers.is_empty() {
            return Ok(TrayStatus::Degraded);
        }
        
        // Additional checks could be added here for other error conditions
        
        Ok(TrayStatus::Healthy)
    }

    async fn update_status_item_title(&self, status_item: id) {
        unsafe {
            let title = if self.config.show_workspace_count {
                let workspace_count = self.workspace_manager.get_workspace_count().await;
                format!("{} TilleRS ({})", self.current_status.icon_char(), workspace_count)
            } else {
                format!("{} TilleRS", self.current_status.icon_char())
            };

            let ns_title = NSString::alloc(nil).init_str(&title);
            let _: () = msg_send![status_item, setTitle: ns_title];
        }
    }

    async fn create_menu(&self) -> Result<id> {
        unsafe {
            let menu = NSMenu::new(nil).autorelease();
            
            // Title item
            let title_item = self.create_menu_item("TilleRS Window Manager", None, nil).await;
            menu.addItem_(title_item);
            
            // Separator
            menu.addItem_(NSMenuItem::separatorItem(nil));
            
            // Status item
            let status_text = format!("Status: {}", self.current_status.description());
            let status_item = self.create_menu_item(&status_text, None, nil).await;
            menu.addItem_(status_item);
            
            // Workspace count
            let workspace_count = self.workspace_manager.get_workspace_count().await;
            let workspace_text = format!("Workspaces: {}", workspace_count);
            let workspace_item = self.create_menu_item(&workspace_text, None, nil).await;
            menu.addItem_(workspace_item);
            
            // Separator
            menu.addItem_(NSMenuItem::separatorItem(nil));
            
            // Actions
            let preferences_item = self.create_menu_item("Preferences...", Some("p"), nil).await;
            menu.addItem_(preferences_item);
            
            let diagnostics_item = self.create_menu_item("Diagnostics...", Some("d"), nil).await;
            menu.addItem_(diagnostics_item);
            
            // Separator
            menu.addItem_(NSMenuItem::separatorItem(nil));
            
            // Quit item
            let quit_item = self.create_menu_item("Quit TilleRS", Some("q"), nil).await;
            menu.addItem_(quit_item);
            
            Ok(menu)
        }
    }

    async fn create_menu_item(&self, title: &str, key_equivalent: Option<&str>, action: id) -> id {
        unsafe {
            let ns_title = NSString::alloc(nil).init_str(title);
            let ns_key = if let Some(key) = key_equivalent {
                NSString::alloc(nil).init_str(key)
            } else {
                NSString::alloc(nil).init_str("")
            };
            
            let item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(ns_title, action, ns_key);
            item.autorelease()
        }
    }

    async fn update_menu_items(&self, menu: id) -> Result<()> {
        // Update dynamic menu items (status, workspace count, etc.)
        // This would involve finding specific menu items and updating their titles
        debug!("Menu items updated");
        Ok(())
    }
}

/// System tray event handler for menu actions
pub struct SystemTrayEventHandler {
    workspace_manager: Arc<WorkspaceManager>,
    error_recovery: Arc<ErrorRecoveryManager>,
}

impl SystemTrayEventHandler {
    pub fn new(
        workspace_manager: Arc<WorkspaceManager>,
        error_recovery: Arc<ErrorRecoveryManager>,
    ) -> Self {
        Self {
            workspace_manager,
            error_recovery,
        }
    }

    /// Handle preferences menu action
    pub async fn handle_preferences(&self) -> Result<()> {
        info!("Opening preferences...");
        
        // In a real implementation, this would open a preferences window
        // For now, we'll just log the action
        debug!("Preferences dialog would open here");
        
        Ok(())
    }

    /// Handle diagnostics menu action
    pub async fn handle_diagnostics(&self) -> Result<()> {
        info!("Opening diagnostics...");
        
        // Get system health information
        let health_status = self.error_recovery.get_health_status().await?;
        let workspace_count = self.workspace_manager.get_workspace_count().await;
        
        // Display diagnostics information
        info!("=== TilleRS Diagnostics ===");
        info!("Permissions granted: {}", health_status.permissions_granted);
        info!("Active circuit breakers: {:?}", health_status.active_circuit_breakers);
        info!("Workspace count: {}", workspace_count);
        info!("Health status: {}", health_status.description());
        
        // In a real implementation, this would show a diagnostics dialog
        debug!("Diagnostics dialog would open here");
        
        Ok(())
    }

    /// Handle quit menu action
    pub async fn handle_quit(&self) -> Result<()> {
        info!("Quit requested from system tray");
        
        // In a real implementation, this would signal the main application to shut down
        // For now, we'll just log the action
        debug!("Application shutdown would be initiated here");
        
        Ok(())
    }
}

/// Create and manage a background task for updating the system tray
pub async fn start_system_tray_update_task(
    mut tray_manager: SystemTrayManager,
) -> Result<()> {
    let update_interval = std::time::Duration::from_secs(tray_manager.config.update_interval_secs);
    
    tokio::spawn(async move {
        loop {
            if let Err(e) = tray_manager.update_status().await {
                warn!("Failed to update system tray status: {}", e);
            }
            
            if let Err(e) = tray_manager.update_visibility().await {
                warn!("Failed to update system tray visibility: {}", e);
            }
            
            tokio::time::sleep(update_interval).await;
        }
    });
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        permissions::PermissionConfig,
        services::{WorkspaceManagerConfig, WindowManagerConfig},
    };

    #[test]
    fn test_tray_status_icons() {
        assert_eq!(TrayStatus::Healthy.icon_char(), "✓");
        assert_eq!(TrayStatus::PermissionsRequired.icon_char(), "⚠");
        assert_eq!(TrayStatus::Degraded.icon_char(), "⚡");
        assert_eq!(TrayStatus::Error.icon_char(), "✗");
    }

    #[test]
    fn test_tray_status_descriptions() {
        assert!(TrayStatus::Healthy.description().contains("running normally"));
        assert!(TrayStatus::PermissionsRequired.description().contains("Permissions"));
        assert!(TrayStatus::Degraded.description().contains("limited"));
        assert!(TrayStatus::Error.description().contains("errors"));
    }

    #[test]
    fn test_system_tray_config() {
        let config = SystemTrayConfig::default();
        assert!(config.show_icon);
        assert!(config.show_workspace_count);
        assert_eq!(config.update_interval_secs, 5);
    }

    #[tokio::test]
    async fn test_system_tray_manager_creation() {
        let config = SystemTrayConfig::default();
        let workspace_manager = Arc::new(WorkspaceManager::new(WorkspaceManagerConfig::default()));
        let window_manager = Arc::new(WindowManager::new(WindowManagerConfig::default()).unwrap());
        let permission_checker = Arc::new(RwLock::new(PermissionChecker::new(PermissionConfig::default())));
        let error_recovery = Arc::new(ErrorRecoveryManager::new(
            crate::error_recovery::RecoveryConfig::default(),
            PermissionChecker::new(PermissionConfig::default()),
        ));
        
        let tray_manager = SystemTrayManager::new(
            config,
            workspace_manager,
            window_manager,
            error_recovery,
            permission_checker,
        );
        
        assert_eq!(tray_manager.current_status, TrayStatus::Healthy);
        assert!(tray_manager.status_item.is_none());
    }
}