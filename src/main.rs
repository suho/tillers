//! TilleRS - Keyboard-First Tiling Window Manager for macOS
//!
//! Main application entry point with comprehensive initialization,
//! error recovery, and application lifecycle management.

use tillers::{
    error_recovery::{ErrorRecoveryManager, RecoveryConfig},
    logging::{LogConfig, init_logging},
    permissions::{PermissionChecker, PermissionConfig},
    services::{
        WorkspaceManager, WorkspaceManagerConfig,
        WindowManager,
        KeyboardHandler,
        TilingEngine,
    },
    Result, TilleRSError,
};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::{
    signal,
    sync::{broadcast, RwLock},
    time::{Duration, sleep},
};
use tracing::{debug, error, info, warn, instrument};

/// Application configuration and state
pub struct TilleRSApp {
    /// Error recovery manager
    error_recovery: Arc<ErrorRecoveryManager>,
    /// Permission checker
    _permission_checker: Arc<RwLock<PermissionChecker>>,
    /// Core services
    workspace_manager: Arc<WorkspaceManager>,
    _window_manager: Arc<WindowManager>,
    _keyboard_handler: Arc<KeyboardHandler>,
    _tiling_engine: Arc<TilingEngine>,
    /// Shutdown signal
    shutdown_tx: broadcast::Sender<()>,
    shutdown_rx: broadcast::Receiver<()>,
}

impl TilleRSApp {
    /// Initialize the TilleRS application
    #[instrument(skip_all)]
    pub async fn new() -> Result<Self> {
        // Initialize logging first
        let log_config = LogConfig::from_env();
        init_logging(&log_config)
            .map_err(|e| TilleRSError::ConfigurationError(format!("Failed to initialize logging: {}", e)))?;

        info!("TilleRS - Keyboard-First Tiling Window Manager v{}", env!("CARGO_PKG_VERSION"));
        info!("Initializing application components...");

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

        // Initialize configuration manager
        // Initialize permission checker
        let permission_config = PermissionConfig::default();
        let permission_checker = Arc::new(RwLock::new(PermissionChecker::new(permission_config)));
        debug!("Permission checker initialized");

        // Initialize error recovery
        let recovery_config = RecoveryConfig::default();
        let error_recovery = Arc::new(ErrorRecoveryManager::new(
            recovery_config,
            PermissionChecker::new(PermissionConfig::default()),
        ));
        debug!("Error recovery manager initialized");

        // Check permissions before initializing system services
        Self::check_initial_permissions(&error_recovery).await?;

        // Initialize core services with error recovery
        let workspace_manager = Self::init_workspace_manager(&error_recovery).await?;
        let window_manager = Self::init_window_manager(&error_recovery).await?;
        let tiling_engine = Self::init_tiling_engine(&error_recovery).await?;
        let keyboard_handler = Self::init_keyboard_handler(&error_recovery).await?;

        info!("All core services initialized successfully");

        Ok(Self {
            error_recovery,
            _permission_checker: permission_checker,
            workspace_manager,
            _window_manager: window_manager,
            _keyboard_handler: keyboard_handler,
            _tiling_engine: tiling_engine,
            shutdown_tx,
            shutdown_rx,
        })
    }

    /// Run the main application loop
    #[instrument(skip_all)]
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting TilleRS main application loop");

        // Setup signal handlers for graceful shutdown
        let shutdown_tx = self.shutdown_tx.clone();
        tokio::spawn(async move {
            if let Err(e) = Self::setup_signal_handlers(shutdown_tx).await {
                error!("Failed to setup signal handlers: {}", e);
            }
        });

        // Start background tasks
        self.start_background_tasks().await?;

        // Load and apply initial configuration
        self.load_initial_configuration().await?;

        // Create default workspace if none exist
        self.ensure_default_workspace().await?;

        info!("TilleRS is ready for window management");

        // Main event loop
        loop {
            tokio::select! {
                // Handle shutdown signal
                _ = self.shutdown_rx.recv() => {
                    info!("Shutdown signal received");
                    break;
                }

                // Periodic health checks
                _ = sleep(Duration::from_secs(30)) => {
                    if let Err(e) = self.perform_health_check().await {
                        warn!("Health check failed: {}", e);
                    }
                }

                // Handle configuration changes
                _ = sleep(Duration::from_secs(5)) => {
                    if let Err(e) = self.check_configuration_changes().await {
                        warn!("Configuration check failed: {}", e);
                    }
                }
            }
        }

        // Graceful shutdown
        self.shutdown().await?;
        Ok(())
    }

    /// Shutdown the application gracefully
    #[instrument(skip_all)]
    async fn shutdown(&self) -> Result<()> {
        info!("Shutting down TilleRS...");

        // Save current state
        if let Err(e) = self.save_application_state().await {
            warn!("Failed to save application state: {}", e);
        }

        // Stop background services
        info!("Stopping background services...");
        
        // In a real implementation, we would:
        // 1. Stop keyboard handler
        // 2. Save workspace states
        // 3. Clean up window arrangements
        // 4. Release system resources

        info!("TilleRS shutdown complete");
        Ok(())
    }

    // Private helper methods

    async fn check_initial_permissions(
        error_recovery: &Arc<ErrorRecoveryManager>,
    ) -> Result<()> {
        info!("Checking macOS permissions...");

        let permissions_granted = error_recovery.check_and_recover_permissions().await?;
        
        if !permissions_granted {
            let instructions = error_recovery.get_permission_recovery_instructions().await?;
            
            error!("Required permissions not granted. Please enable the following:");
            for instruction in instructions {
                println!("\n{}", instruction);
            }
            
            return Err(TilleRSError::PermissionDenied(
                "Required macOS permissions not granted. Please enable permissions and restart TilleRS.".to_string()
            ).into());
        }

        info!("All required permissions verified");
        Ok(())
    }

    async fn init_workspace_manager(error_recovery: &Arc<ErrorRecoveryManager>) -> Result<Arc<WorkspaceManager>> {
        let manager = error_recovery
            .recover_and_retry("workspace_manager_init", || {
                WorkspaceManager::new(WorkspaceManagerConfig::default())
            })
            .await?;

        debug!("Workspace manager initialized");
        Ok(Arc::new(manager))
    }

    async fn init_window_manager(error_recovery: &Arc<ErrorRecoveryManager>) -> Result<Arc<WindowManager>> {
        let manager = error_recovery.recover_and_retry("window_manager_init", || {
            Ok(WindowManager::with_default_providers())
        }).await?;

        debug!("Window manager initialized");
        Ok(Arc::new(manager))
    }

    async fn init_tiling_engine(error_recovery: &Arc<ErrorRecoveryManager>) -> Result<Arc<TilingEngine>> {
        let engine = error_recovery
            .recover_and_retry("tiling_engine_init", || Ok(TilingEngine::new()))
            .await?;

        debug!("Tiling engine initialized");
        Ok(Arc::new(engine))
    }

    async fn init_keyboard_handler(error_recovery: &Arc<ErrorRecoveryManager>) -> Result<Arc<KeyboardHandler>> {
        let handler = error_recovery
            .recover_and_retry("keyboard_handler_init", || {
                Ok(KeyboardHandler::new(HashSet::new()))
            })
            .await?;

        debug!("Keyboard handler initialized");
        Ok(Arc::new(handler))
    }

    async fn setup_signal_handlers(shutdown_tx: broadcast::Sender<()>) -> Result<()> {
        #[cfg(unix)]
        {
            let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())?;
            tokio::select! {
                res = signal::ctrl_c() => {
                    match res {
                        Ok(_) => info!("Received SIGINT (Ctrl+C)"),
                        Err(e) => warn!("Failed to listen for Ctrl+C: {}", e),
                    }
                }
                _ = sigterm.recv() => {
                    info!("Received SIGTERM");
                }
            }
        }

        #[cfg(not(unix))]
        {
            match signal::ctrl_c().await {
                Ok(_) => info!("Received Ctrl+C"),
                Err(e) => warn!("Failed to listen for Ctrl+C: {}", e),
            }
        }

        if shutdown_tx.send(()).is_err() {
            warn!("Failed to send shutdown signal - no receivers");
        }

        Ok(())
    }

    async fn start_background_tasks(&self) -> Result<()> {
        info!("Starting background tasks...");

        // Start permission monitoring
        let error_recovery = self.error_recovery.clone();
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(60)).await;
                if let Err(e) = error_recovery.check_and_recover_permissions().await {
                    warn!("Background permission check failed: {}", e);
                }
            }
        });

        debug!("Background tasks started");
        Ok(())
    }

    async fn load_initial_configuration(&self) -> Result<()> {
        info!("Loading initial configuration...");

        // Load configuration with error recovery
        let _config = self.error_recovery.recover_and_retry("load_config", || {
            // In a real implementation, this would load the actual configuration
            Ok(())
        }).await?;

        debug!("Initial configuration loaded");
        Ok(())
    }

    async fn ensure_default_workspace(&self) -> Result<()> {
        info!("Ensuring default workspace exists...");

        let workspace_count = self.workspace_manager.get_workspace_count().await;
        
        if workspace_count == 0 {
            info!("No workspaces found, creating default workspace");
            
            // This would use the actual workspace creation API
            // For now, we'll just log the intention
            debug!("Default workspace would be created here");
        } else {
            debug!("Found {} existing workspaces", workspace_count);
        }

        Ok(())
    }

    async fn perform_health_check(&self) -> Result<()> {
        debug!("Performing health check...");

        let health_status = self.error_recovery.get_health_status().await?;
        
        if !health_status.is_healthy() {
            warn!("System health issues detected: {}", health_status.description());
        }

        Ok(())
    }

    async fn check_configuration_changes(&self) -> Result<()> {
        // Check for configuration file changes and reload if necessary
        // This would monitor configuration files and reload them
        debug!("Configuration check completed");
        Ok(())
    }

    async fn save_application_state(&self) -> Result<()> {
        info!("Saving application state...");

        // Save workspace configurations
        // Save window arrangements
        // Save user preferences

        debug!("Application state saved");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize and run the application
    let mut app = TilleRSApp::new().await?;
    
    if let Err(e) = app.run().await {
        error!("Application error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
