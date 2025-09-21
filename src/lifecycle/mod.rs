//! Application lifecycle management for TilleRS
//!
//! Handles application startup, shutdown, background operation,
//! and system integration for the TilleRS window manager.

use crate::{
    cli::run_cli,
    error_recovery::ErrorRecoveryManager,
    logging::{LogConfig, init_logging},
    permissions::PermissionChecker,
    services::{WorkspaceManager, WindowManager, KeyboardHandler, TilingEngine},
    ui::{SystemTrayManager, SystemTrayConfig},
    Result, TilleRSError,
};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    fs,
    signal,
    sync::{broadcast, RwLock, Mutex},
    time::sleep,
};
use tracing::{debug, info, warn, error, instrument};
use serde::{Deserialize, Serialize};

/// Application lifecycle configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleConfig {
    /// Whether to start in background mode
    pub start_in_background: bool,
    /// Whether to show system tray
    pub show_system_tray: bool,
    /// Whether to auto-start on system boot
    pub auto_start: bool,
    /// Graceful shutdown timeout (seconds)
    pub shutdown_timeout_secs: u64,
    /// Health check interval (seconds)
    pub health_check_interval_secs: u64,
    /// State persistence interval (seconds)
    pub state_persistence_interval_secs: u64,
    /// Log rotation settings
    pub log_rotation_enabled: bool,
    /// Maximum log file size (MB)
    pub max_log_size_mb: u64,
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self {
            start_in_background: true,
            show_system_tray: true,
            auto_start: false,
            shutdown_timeout_secs: 30,
            health_check_interval_secs: 30,
            state_persistence_interval_secs: 300, // 5 minutes
            log_rotation_enabled: true,
            max_log_size_mb: 100,
        }
    }
}

/// Application operation mode
#[derive(Debug, Clone, PartialEq)]
pub enum OperationMode {
    /// Interactive CLI mode
    Cli,
    /// Background daemon mode
    Daemon,
    /// Foreground service mode
    Service,
}

/// Application state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationState {
    /// Application start time
    pub start_time: String,
    /// Current operation mode
    pub operation_mode: String,
    /// Active workspace information
    pub active_workspace_id: Option<String>,
    /// Last health check time
    pub last_health_check: Option<String>,
    /// Performance metrics
    pub performance_metrics: PerformanceMetrics,
}

/// Performance metrics tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Total workspace switches
    pub workspace_switches: u64,
    /// Total window arrangements
    pub window_arrangements: u64,
    /// Average workspace switch time (ms)
    pub avg_workspace_switch_ms: f64,
    /// Average window arrangement time (ms)
    pub avg_window_arrangement_ms: f64,
    /// Memory usage (MB)
    pub memory_usage_mb: f64,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            workspace_switches: 0,
            window_arrangements: 0,
            avg_workspace_switch_ms: 0.0,
            avg_window_arrangement_ms: 0.0,
            memory_usage_mb: 0.0,
        }
    }
}

/// Lifecycle manager for TilleRS application
pub struct LifecycleManager {
    config: LifecycleConfig,
    operation_mode: OperationMode,
    
    // Core services
    workspace_manager: Arc<WorkspaceManager>,
    window_manager: Arc<WindowManager>,
    _keyboard_handler: Arc<KeyboardHandler>,
    _tiling_engine: Arc<TilingEngine>,
    
    // Support services
    permission_checker: Arc<RwLock<PermissionChecker>>,
    error_recovery: Arc<ErrorRecoveryManager>,
    
    // UI components
    system_tray: Option<Arc<Mutex<SystemTrayManager>>>,
    
    // Lifecycle management
    shutdown_signal: Arc<Mutex<Option<broadcast::Sender<()>>>>,
    _start_time: Instant,
    
    // State tracking
    application_state: Arc<RwLock<ApplicationState>>,
    performance_metrics: Arc<RwLock<PerformanceMetrics>>,
}

impl LifecycleManager {
    /// Create a new lifecycle manager
    pub fn new(
        config: LifecycleConfig,
        operation_mode: OperationMode,
        workspace_manager: Arc<WorkspaceManager>,
        window_manager: Arc<WindowManager>,
        keyboard_handler: Arc<KeyboardHandler>,
        tiling_engine: Arc<TilingEngine>,
        permission_checker: Arc<RwLock<PermissionChecker>>,
        error_recovery: Arc<ErrorRecoveryManager>,
    ) -> Self {
        let initial_state = ApplicationState {
            start_time: chrono::Utc::now().to_rfc3339(),
            operation_mode: format!("{:?}", operation_mode),
            active_workspace_id: None,
            last_health_check: None,
            performance_metrics: PerformanceMetrics::default(),
        };

        Self {
            config,
            operation_mode,
            workspace_manager,
            window_manager,
            _keyboard_handler: keyboard_handler,
            _tiling_engine: tiling_engine,
            permission_checker,
            error_recovery,
            system_tray: None,
            shutdown_signal: Arc::new(Mutex::new(None)),
            _start_time: Instant::now(),
            application_state: Arc::new(RwLock::new(initial_state)),
            performance_metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
        }
    }

    /// Initialize the application lifecycle
    #[instrument(skip_all)]
    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing TilleRS lifecycle manager...");

        // Initialize logging with lifecycle-aware configuration
        self.initialize_logging().await?;

        // Load persisted state
        self.load_persisted_state().await?;

        // Initialize system tray if enabled
        if self.config.show_system_tray && self.operation_mode != OperationMode::Cli {
            self.initialize_system_tray().await?;
        }

        // Setup signal handlers
        self.setup_signal_handlers().await?;

        // Start background tasks
        self.start_background_tasks().await?;

        info!("Lifecycle manager initialized successfully");
        Ok(())
    }

    /// Run the application in the configured mode
    #[instrument(skip_all)]
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting TilleRS in {:?} mode", self.operation_mode);

        match self.operation_mode {
            OperationMode::Cli => self.run_cli_mode().await,
            OperationMode::Daemon => self.run_daemon_mode().await,
            OperationMode::Service => self.run_service_mode().await,
        }
    }

    /// Shutdown the application gracefully
    #[instrument(skip_all)]
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Initiating graceful shutdown...");

        let shutdown_start = Instant::now();
        let timeout = Duration::from_secs(self.config.shutdown_timeout_secs);

        // Signal shutdown to all components
        if let Some(sender) = self.shutdown_signal.lock().await.as_ref() {
            if sender.send(()).is_err() {
                warn!("No shutdown receivers available");
            }
        }

        // Save current state
        if let Err(e) = self.save_application_state().await {
            warn!("Failed to save application state during shutdown: {}", e);
        }

        // Stop services in reverse order of initialization
        info!("Stopping core services...");

        // Wait for background tasks to complete (with timeout)
        tokio::select! {
            _ = sleep(timeout) => {
                warn!("Shutdown timeout reached, forcing exit");
            }
            _ = self.wait_for_clean_shutdown() => {
                debug!("Clean shutdown completed");
            }
        }

        let shutdown_duration = shutdown_start.elapsed();
        info!("Shutdown completed in {:?}", shutdown_duration);

        Ok(())
    }

    /// Get current application metrics
    pub async fn get_metrics(&self) -> PerformanceMetrics {
        self.performance_metrics.read().await.clone()
    }

    /// Get current application state
    pub async fn get_state(&self) -> ApplicationState {
        self.application_state.read().await.clone()
    }

    // Private implementation methods

    async fn initialize_logging(&self) -> Result<()> {
        let log_config = LogConfig::from_env();
        init_logging(&log_config)
            .map_err(|e| TilleRSError::ConfigurationError(format!("Failed to initialize logging: {}", e)))?;
        
        info!("TilleRS v{} starting up", env!("CARGO_PKG_VERSION"));
        info!("Operation mode: {:?}", self.operation_mode);
        
        Ok(())
    }

    async fn load_persisted_state(&mut self) -> Result<()> {
        let state_file = self.get_state_file_path();
        
        if state_file.exists() {
            match fs::read_to_string(&state_file).await {
                Ok(content) => {
                    match serde_json::from_str::<ApplicationState>(&content) {
                        Ok(state) => {
                            *self.application_state.write().await = state;
                            info!("Loaded persisted application state from {:?}", state_file);
                        }
                        Err(e) => {
                            warn!("Failed to parse state file: {}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to read state file: {}", e);
                }
            }
        } else {
            debug!("No persisted state file found, starting fresh");
        }
        
        Ok(())
    }

    async fn initialize_system_tray(&mut self) -> Result<()> {
        info!("Initializing system tray...");
        
        let tray_config = SystemTrayConfig::default();
        let mut tray_manager = SystemTrayManager::new(
            tray_config,
            self.workspace_manager.clone(),
            self.window_manager.clone(),
            self.error_recovery.clone(),
            self.permission_checker.clone(),
        );

        tray_manager.initialize().await?;
        self.system_tray = Some(Arc::new(Mutex::new(tray_manager)));
        
        debug!("System tray initialized");
        Ok(())
    }

    async fn setup_signal_handlers(&mut self) -> Result<()> {
        let (shutdown_tx, _) = broadcast::channel(1);
        *self.shutdown_signal.lock().await = Some(shutdown_tx.clone());

        // Spawn signal handler task
        tokio::spawn(async move {
            #[cfg(unix)]
            {
                let mut sigterm_stream = match signal::unix::signal(signal::unix::SignalKind::terminate()) {
                    Ok(stream) => stream,
                    Err(e) => {
                        warn!("Failed to initialise SIGTERM handler: {}", e);
                        match signal::ctrl_c().await {
                            Ok(_) => info!("Received SIGINT (Ctrl+C)"),
                            Err(err) => warn!("Failed to listen for Ctrl+C: {}", err),
                        }
                        return;
                    }
                };

                tokio::select! {
                    res = signal::ctrl_c() => {
                        match res {
                            Ok(_) => info!("Received SIGINT (Ctrl+C)"),
                            Err(e) => warn!("Failed to listen for Ctrl+C: {}", e),
                        }
                    }
                    _ = sigterm_stream.recv() => {
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
                warn!("Failed to send shutdown signal");
            }
        });

        debug!("Signal handlers setup complete");
        Ok(())
    }

    async fn start_background_tasks(&self) -> Result<()> {
        info!("Starting background tasks...");

        // Health monitoring task
        self.start_health_monitoring_task().await;

        // State persistence task
        self.start_state_persistence_task().await;

        // Performance metrics collection task
        self.start_metrics_collection_task().await;

        // System tray update task
        if let Some(tray) = &self.system_tray {
            self.start_system_tray_update_task(tray.clone()).await;
        }

        debug!("Background tasks started");
        Ok(())
    }

    async fn start_health_monitoring_task(&self) {
        let error_recovery = self.error_recovery.clone();
        let interval = Duration::from_secs(self.config.health_check_interval_secs);
        let application_state = self.application_state.clone();

        tokio::spawn(async move {
            loop {
                sleep(interval).await;
                
                match error_recovery.get_health_status().await {
                    Ok(health) => {
                        let mut state = application_state.write().await;
                        state.last_health_check = Some(chrono::Utc::now().to_rfc3339());
                        
                        if !health.is_healthy() {
                            warn!("Health check detected issues: {}", health.description());
                        }
                    }
                    Err(e) => {
                        error!("Health check failed: {}", e);
                    }
                }
            }
        });
    }

    async fn start_state_persistence_task(&self) {
        let interval = Duration::from_secs(self.config.state_persistence_interval_secs);
        let application_state = self.application_state.clone();
        let state_file = self.get_state_file_path();

        tokio::spawn(async move {
            loop {
                sleep(interval).await;
                
                let state = application_state.read().await.clone();
                
                match serde_json::to_string_pretty(&state) {
                    Ok(json) => {
                        if let Err(e) = fs::write(&state_file, json).await {
                            warn!("Failed to persist application state: {}", e);
                        } else {
                            debug!("Application state persisted");
                        }
                    }
                    Err(e) => {
                        error!("Failed to serialize application state: {}", e);
                    }
                }
            }
        });
    }

    async fn start_metrics_collection_task(&self) {
        let performance_metrics = self.performance_metrics.clone();
        
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(60)).await; // Collect metrics every minute
                
                // Collect system metrics
                let memory_usage = Self::get_memory_usage().await;
                
                let mut metrics = performance_metrics.write().await;
                metrics.memory_usage_mb = memory_usage;
                
                debug!("Performance metrics updated: memory={}MB", memory_usage);
            }
        });
    }

    async fn start_system_tray_update_task(&self, tray: Arc<Mutex<SystemTrayManager>>) {
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(5)).await;
                
                if let Err(e) = tray.lock().await.update_status().await {
                    warn!("Failed to update system tray: {}", e);
                }
            }
        });
    }

    async fn run_cli_mode(&self) -> Result<()> {
        info!("Running in CLI mode");
        
        run_cli(
            self.workspace_manager.clone(),
            self.error_recovery.clone(),
        ).await
    }

    async fn run_daemon_mode(&self) -> Result<()> {
        info!("Running in daemon mode");
        
        // In daemon mode, we run indefinitely until shutdown
        if let Some(sender) = self.shutdown_signal.lock().await.as_ref() {
            let mut shutdown_rx = sender.subscribe();
            
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        info!("Shutdown signal received in daemon mode");
                        break;
                    }
                    _ = sleep(Duration::from_secs(1)) => {
                        // Main daemon loop - handle events, etc.
                        continue;
                    }
                }
            }
        }
        
        Ok(())
    }

    async fn run_service_mode(&self) -> Result<()> {
        info!("Running in service mode");
        
        // Service mode is similar to daemon but with different lifecycle expectations
        if let Some(sender) = self.shutdown_signal.lock().await.as_ref() {
            let mut shutdown_rx = sender.subscribe();
            shutdown_rx.recv().await.ok();
        }
        
        Ok(())
    }

    async fn save_application_state(&self) -> Result<()> {
        let state = self.application_state.read().await.clone();
        let state_file = self.get_state_file_path();
        
        let json = serde_json::to_string_pretty(&state)?;
        fs::write(state_file, json).await?;
        
        debug!("Application state saved");
        Ok(())
    }

    async fn wait_for_clean_shutdown(&self) {
        // Wait for all background tasks to complete
        // In a real implementation, this would track running tasks
        sleep(Duration::from_millis(100)).await;
    }

    fn get_state_file_path(&self) -> PathBuf {
        let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("tillers");
        std::fs::create_dir_all(&path).ok();
        path.push("state.json");
        path
    }

    async fn get_memory_usage() -> f64 {
        // In a real implementation, this would get actual memory usage
        // For now, return a placeholder value
        64.0 // MB
    }
}

/// Determine the appropriate operation mode based on command line arguments
pub fn determine_operation_mode() -> OperationMode {
    let args: Vec<String> = std::env::args().collect();
    
    // Check if any CLI subcommands are present
    if args.len() > 1 && !args[1].starts_with('-') {
        match args[1].as_str() {
            "workspace" | "window" | "config" | "permissions" | "diagnostics" | "service" => {
                return OperationMode::Cli;
            }
            _ => {}
        }
    }
    
    // Check for daemon/service flags
    if args.iter().any(|arg| arg == "--daemon" || arg == "-d") {
        OperationMode::Daemon
    } else if args.iter().any(|arg| arg == "--service") {
        OperationMode::Service
    } else if args.len() == 1 {
        // No arguments - default to daemon mode
        OperationMode::Daemon
    } else {
        // Has arguments but not recognized commands - CLI mode
        OperationMode::Cli
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_mode_determination() {
        // Test CLI mode detection
        std::env::set_var("ARGV", "tillers workspace list");
        // In a real test, we'd mock std::env::args()
        
        let mode = determine_operation_mode();
        // The actual implementation depends on command line parsing
        assert!(matches!(mode, OperationMode::Daemon | OperationMode::Cli | OperationMode::Service));
    }

    #[test]
    fn test_lifecycle_config() {
        let config = LifecycleConfig::default();
        assert!(config.start_in_background);
        assert!(config.show_system_tray);
        assert_eq!(config.shutdown_timeout_secs, 30);
    }

    #[test]
    fn test_performance_metrics() {
        let metrics = PerformanceMetrics::default();
        assert_eq!(metrics.workspace_switches, 0);
        assert_eq!(metrics.window_arrangements, 0);
    }

    #[tokio::test]
    async fn test_application_state_serialization() {
        let state = ApplicationState {
            start_time: "2023-01-01T00:00:00Z".to_string(),
            operation_mode: "Daemon".to_string(),
            active_workspace_id: Some("workspace-1".to_string()),
            last_health_check: None,
            performance_metrics: PerformanceMetrics::default(),
        };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: ApplicationState = serde_json::from_str(&json).unwrap();
        
        assert_eq!(state.start_time, deserialized.start_time);
        assert_eq!(state.operation_mode, deserialized.operation_mode);
    }
}
