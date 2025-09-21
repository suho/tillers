//! Command-line interface for TilleRS
//!
//! Provides CLI commands for configuration management, debugging,
//! and system administration of the TilleRS window manager.

use crate::{error_recovery::ErrorRecoveryManager, services::WorkspaceManager, Result};
use clap::{Args, Parser, Subcommand};
use serde_json;
use std::env;
use std::sync::Arc;
use tracing::{debug, error, info};

/// TilleRS command-line interface
#[derive(Parser)]
#[command(name = "tillers")]
#[command(about = "Keyboard-first tiling window manager for macOS")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(author = "TilleRS Team")]
pub struct TilleRSCli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Configuration file path
    #[arg(short, long, global = true)]
    pub config: Option<String>,

    /// Enable JSON output for machine-readable results
    #[arg(long, global = true)]
    pub json: bool,

    /// Subcommands
    #[command(subcommand)]
    pub command: Commands,
}

/// Available CLI commands
#[derive(Subcommand)]
pub enum Commands {
    /// Workspace management commands
    Workspace(WorkspaceCommands),

    /// Window management commands
    Window(WindowCommands),

    /// Configuration management commands
    Config(ConfigCommands),

    /// Permission management commands
    Permissions(PermissionCommands),

    /// Diagnostics and debugging commands
    Diagnostics(DiagnosticsCommands),

    /// Service management commands
    Service(ServiceCommands),
}

/// Workspace management subcommands
#[derive(Args)]
pub struct WorkspaceCommands {
    #[command(subcommand)]
    pub action: WorkspaceActions,
}

#[derive(Subcommand)]
pub enum WorkspaceActions {
    /// List all workspaces
    List,

    /// Create a new workspace
    Create {
        /// Workspace name
        name: String,

        /// Workspace description
        #[arg(short, long)]
        description: Option<String>,

        /// Keyboard shortcut (e.g., "opt+1")
        #[arg(short, long)]
        shortcut: Option<String>,

        /// Auto-arrange windows
        #[arg(short, long)]
        auto_arrange: bool,
    },

    /// Delete a workspace
    Delete {
        /// Workspace name or ID
        workspace: String,

        /// Force deletion without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Switch to a workspace
    Switch {
        /// Workspace name or ID
        workspace: String,
    },

    /// Show workspace details
    Show {
        /// Workspace name or ID
        workspace: String,
    },
}

/// Window management subcommands
#[derive(Args)]
pub struct WindowCommands {
    #[command(subcommand)]
    pub action: WindowActions,
}

#[derive(Subcommand)]
pub enum WindowActions {
    /// List all windows
    List,

    /// Show window details
    Show {
        /// Window ID
        window_id: u32,
    },

    /// Move window to workspace
    Move {
        /// Window ID
        window_id: u32,

        /// Target workspace name or ID
        workspace: String,
    },

    /// Apply tiling pattern to windows
    Tile {
        /// Tiling pattern name
        pattern: String,

        /// Workspace name (optional, defaults to current)
        #[arg(short, long)]
        workspace: Option<String>,
    },
}

/// Configuration management subcommands
#[derive(Args)]
pub struct ConfigCommands {
    #[command(subcommand)]
    pub action: ConfigActions,
}

#[derive(Subcommand)]
pub enum ConfigActions {
    /// Show current configuration
    Show,

    /// Validate configuration
    Validate {
        /// Configuration file path
        #[arg(short, long)]
        file: Option<String>,
    },

    /// Set configuration value
    Set {
        /// Configuration key (dot notation, e.g., "keyboard.modifier")
        key: String,

        /// Configuration value
        value: String,
    },

    /// Get configuration value
    Get {
        /// Configuration key (dot notation)
        key: String,
    },

    /// Reset configuration to defaults
    Reset {
        /// Force reset without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Export configuration
    Export {
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Import configuration
    Import {
        /// Input file path
        file: String,

        /// Merge with existing configuration
        #[arg(short, long)]
        merge: bool,
    },
}

/// Permission management subcommands
#[derive(Args)]
pub struct PermissionCommands {
    #[command(subcommand)]
    pub action: PermissionActions,
}

#[derive(Subcommand)]
pub enum PermissionActions {
    /// Check permission status
    Status,

    /// Request permissions
    Request {
        /// Specific permission type (accessibility, input-monitoring, screen-recording)
        #[arg(short, long)]
        permission: Option<String>,
    },

    /// Show permission instructions
    Instructions,
}

/// Diagnostics and debugging subcommands
#[derive(Args)]
pub struct DiagnosticsCommands {
    #[command(subcommand)]
    pub action: DiagnosticsActions,
}

#[derive(Subcommand)]
pub enum DiagnosticsActions {
    /// Show system health status
    Health,

    /// Show detailed system information
    System,

    /// Check API connectivity
    ApiCheck,

    /// Export logs
    Logs {
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,

        /// Number of recent log lines to export
        #[arg(short, long, default_value = "1000")]
        lines: usize,
    },

    /// Performance benchmark
    Benchmark {
        /// Benchmark type (workspace-switching, window-positioning)
        benchmark_type: String,

        /// Number of iterations
        #[arg(short, long, default_value = "100")]
        iterations: usize,
    },
}

/// Service management subcommands
#[derive(Args)]
pub struct ServiceCommands {
    #[command(subcommand)]
    pub action: ServiceActions,
}

#[derive(Subcommand)]
pub enum ServiceActions {
    /// Start TilleRS service
    Start,

    /// Stop TilleRS service
    Stop,

    /// Restart TilleRS service
    Restart,

    /// Show service status
    Status,

    /// Install TilleRS as a system service
    Install,

    /// Uninstall TilleRS system service
    Uninstall,
}

/// CLI command executor
pub struct TilleRSCliExecutor {
    workspace_manager: Arc<WorkspaceManager>,
    error_recovery: Arc<ErrorRecoveryManager>,
    json_output: bool,
}

impl TilleRSCliExecutor {
    /// Create a new CLI executor
    pub fn new(
        workspace_manager: Arc<WorkspaceManager>,
        error_recovery: Arc<ErrorRecoveryManager>,
        json_output: bool,
    ) -> Self {
        Self {
            workspace_manager,
            error_recovery,
            json_output,
        }
    }

    /// Execute a CLI command
    pub async fn execute(&self, command: Commands) -> Result<()> {
        match command {
            Commands::Workspace(workspace_cmd) => {
                self.execute_workspace_command(workspace_cmd).await
            }
            Commands::Window(window_cmd) => self.execute_window_command(window_cmd).await,
            Commands::Config(config_cmd) => self.execute_config_command(config_cmd).await,
            Commands::Permissions(permission_cmd) => {
                self.execute_permission_command(permission_cmd).await
            }
            Commands::Diagnostics(diagnostics_cmd) => {
                self.execute_diagnostics_command(diagnostics_cmd).await
            }
            Commands::Service(service_cmd) => self.execute_service_command(service_cmd).await,
        }
    }

    // Workspace command implementations

    async fn execute_workspace_command(&self, cmd: WorkspaceCommands) -> Result<()> {
        match cmd.action {
            WorkspaceActions::List => {
                let workspaces = self.workspace_manager.list_workspaces().await;

                if self.json_output {
                    println!("{}", serde_json::to_string_pretty(&workspaces)?);
                } else if workspaces.is_empty() {
                    println!("No workspaces found.");
                } else {
                    println!("Workspaces:");
                    for workspace in workspaces {
                        println!(
                            "  {} - {} ({})",
                            workspace.id,
                            workspace.name,
                            if workspace.is_active() {
                                "active"
                            } else {
                                "inactive"
                            }
                        );
                    }
                }
            }
            WorkspaceActions::Create {
                name,
                description,
                shortcut,
                auto_arrange,
            } => {
                info!("Creating workspace: {}", name);

                // This would use the actual workspace creation API
                println!("Workspace '{}' would be created with:", name);
                if let Some(desc) = description {
                    println!("  Description: {}", desc);
                }
                if let Some(sc) = shortcut {
                    println!("  Shortcut: {}", sc);
                }
                println!("  Auto-arrange: {}", auto_arrange);
            }
            WorkspaceActions::Delete { workspace, force } => {
                if !force {
                    println!(
                        "Are you sure you want to delete workspace '{}'? (y/N)",
                        workspace
                    );
                    // In a real implementation, this would wait for user confirmation
                }

                info!("Deleting workspace: {}", workspace);
                println!("Workspace '{}' would be deleted", workspace);
            }
            WorkspaceActions::Switch { workspace } => {
                info!("Switching to workspace: {}", workspace);
                println!("Would switch to workspace '{}'", workspace);
            }
            WorkspaceActions::Show { workspace } => {
                info!("Showing workspace details: {}", workspace);
                println!(
                    "Workspace details for '{}' would be displayed here",
                    workspace
                );
            }
        }

        Ok(())
    }

    // Window command implementations

    async fn execute_window_command(&self, cmd: WindowCommands) -> Result<()> {
        match cmd.action {
            WindowActions::List => {
                info!("Listing all windows");

                if self.json_output {
                    println!("{{\"windows\": []}}");
                } else {
                    println!("Windows:");
                    println!("  (Window list would be displayed here)");
                }
            }
            WindowActions::Show { window_id } => {
                info!("Showing window details for ID: {}", window_id);
                println!("Window {} details would be displayed here", window_id);
            }
            WindowActions::Move {
                window_id,
                workspace,
            } => {
                info!("Moving window {} to workspace {}", window_id, workspace);
                println!(
                    "Would move window {} to workspace '{}'",
                    window_id, workspace
                );
            }
            WindowActions::Tile { pattern, workspace } => {
                let ws_name = workspace.unwrap_or_else(|| "current".to_string());
                info!(
                    "Applying tiling pattern '{}' to workspace '{}'",
                    pattern, ws_name
                );
                println!(
                    "Would apply tiling pattern '{}' to workspace '{}'",
                    pattern, ws_name
                );
            }
        }

        Ok(())
    }

    // Configuration command implementations

    async fn execute_config_command(&self, cmd: ConfigCommands) -> Result<()> {
        match cmd.action {
            ConfigActions::Show => {
                info!("Displaying current configuration");

                if self.json_output {
                    println!("{{\"config\": {{\"placeholder\": true}}}}");
                } else {
                    println!("Current Configuration:");
                    println!("  (Configuration would be displayed here)");
                }
            }
            ConfigActions::Validate { file } => {
                let file_path = file.unwrap_or_else(|| "default config".to_string());
                info!("Validating configuration: {}", file_path);
                println!(
                    "Configuration validation would be performed for: {}",
                    file_path
                );
            }
            ConfigActions::Set { key, value } => {
                info!("Setting configuration: {} = {}", key, value);
                println!("Would set {} = {}", key, value);
            }
            ConfigActions::Get { key } => {
                info!("Getting configuration value: {}", key);
                println!("Value for '{}' would be displayed here", key);
            }
            ConfigActions::Reset { force } => {
                if !force {
                    println!("Are you sure you want to reset configuration to defaults? (y/N)");
                }
                info!("Resetting configuration to defaults");
                println!("Configuration would be reset to defaults");
            }
            ConfigActions::Export { output } => {
                let output_path = output.unwrap_or_else(|| "stdout".to_string());
                info!("Exporting configuration to: {}", output_path);
                println!("Configuration would be exported to: {}", output_path);
            }
            ConfigActions::Import { file, merge } => {
                info!("Importing configuration from: {} (merge: {})", file, merge);
                println!(
                    "Would import configuration from: {} (merge: {})",
                    file, merge
                );
            }
        }

        Ok(())
    }

    // Permission command implementations

    async fn execute_permission_command(&self, cmd: PermissionCommands) -> Result<()> {
        match cmd.action {
            PermissionActions::Status => {
                info!("Checking permission status");

                let health_status = self.error_recovery.get_health_status().await?;

                if self.json_output {
                    let breaker_info = if health_status.active_circuit_breakers.is_empty() {
                        Vec::new()
                    } else {
                        health_status.active_circuit_breakers.clone()
                    };
                    let last_check_secs = health_status
                        .last_permission_check
                        .map(|instant| instant.elapsed().as_secs_f64());
                    let status_json = serde_json::json!({
                        "permissions_granted": health_status.permissions_granted,
                        "active_circuit_breakers": breaker_info,
                        "last_check_secs": last_check_secs,
                        "description": health_status.description(),
                    });
                    println!("{}", serde_json::to_string_pretty(&status_json)?);
                } else {
                    println!("Permission Status:");
                    println!(
                        "  Permissions granted: {}",
                        health_status.permissions_granted
                    );
                    println!(
                        "  Active circuit breakers: {:?}",
                        health_status.active_circuit_breakers
                    );
                    println!("  System health: {}", health_status.description());
                }
            }
            PermissionActions::Request { permission } => match permission {
                Some(perm_type) => {
                    info!("Requesting specific permission: {}", perm_type);
                    println!("Would request {} permission", perm_type);
                }
                None => {
                    info!("Requesting all required permissions");
                    let granted = self.error_recovery.check_and_recover_permissions().await?;

                    if granted {
                        println!("All required permissions are granted");
                    } else {
                        println!("Some permissions are missing - please check System Preferences");
                    }
                }
            },
            PermissionActions::Instructions => {
                info!("Displaying permission instructions");

                let instructions = self
                    .error_recovery
                    .get_permission_recovery_instructions()
                    .await?;

                if instructions.is_empty() {
                    println!("All required permissions are granted!");
                } else {
                    println!("Permission Setup Instructions:");
                    for (i, instruction) in instructions.iter().enumerate() {
                        println!("\n{}. {}", i + 1, instruction);
                    }
                }
            }
        }

        Ok(())
    }

    // Diagnostics command implementations

    async fn execute_diagnostics_command(&self, cmd: DiagnosticsCommands) -> Result<()> {
        match cmd.action {
            DiagnosticsActions::Health => {
                info!("Performing health check");

                let health_status = self.error_recovery.get_health_status().await?;
                let workspace_count = self.workspace_manager.get_workspace_count().await;

                if self.json_output {
                    let breaker_list = &health_status.active_circuit_breakers;
                    let breaker_info = if breaker_list.is_empty() {
                        Vec::new()
                    } else {
                        breaker_list.clone()
                    };
                    let last_check_secs = health_status
                        .last_permission_check
                        .map(|instant| instant.elapsed().as_secs_f64());

                    let health_json = serde_json::json!({
                        "healthy": health_status.is_healthy(),
                        "permissions_granted": health_status.permissions_granted,
                        "active_circuit_breakers": breaker_info,
                        "last_permission_check_secs": last_check_secs,
                        "workspace_count": workspace_count,
                        "description": health_status.description()
                    });
                    println!("{}", serde_json::to_string_pretty(&health_json)?);
                } else {
                    println!("=== TilleRS Health Check ===");
                    println!(
                        "Status: {}",
                        if health_status.is_healthy() {
                            "HEALTHY"
                        } else {
                            "UNHEALTHY"
                        }
                    );
                    println!(
                        "Permissions: {}",
                        if health_status.permissions_granted {
                            "OK"
                        } else {
                            "MISSING"
                        }
                    );
                    let circuit_breakers = if health_status.active_circuit_breakers.is_empty() {
                        "None".to_string()
                    } else {
                        format!("{:?}", health_status.active_circuit_breakers)
                    };
                    println!("Circuit breakers: {}", circuit_breakers);
                    println!("Workspaces: {}", workspace_count);
                    println!("Description: {}", health_status.description());
                }
            }
            DiagnosticsActions::System => {
                info!("Gathering system information");
                println!("=== System Information ===");
                println!("TilleRS version: {}", env!("CARGO_PKG_VERSION"));
                let rust_version = option_env!("RUSTC_VERSION")
                    .map(str::to_string)
                    .or_else(|| env::var("RUSTC_VERSION").ok())
                    .unwrap_or_else(|| "unknown".to_string());
                println!("Rust version: {}", rust_version);
                println!("Target OS: {}", std::env::consts::OS);
                println!("Target architecture: {}", std::env::consts::ARCH);
            }
            DiagnosticsActions::ApiCheck => {
                info!("Checking API connectivity");
                println!("API connectivity check would be performed here");
            }
            DiagnosticsActions::Logs { output, lines } => {
                let output_path = output.unwrap_or_else(|| "stdout".to_string());
                info!("Exporting {} log lines to: {}", lines, output_path);
                println!("Would export {} log lines to: {}", lines, output_path);
            }
            DiagnosticsActions::Benchmark {
                benchmark_type,
                iterations,
            } => {
                info!(
                    "Running benchmark: {} ({} iterations)",
                    benchmark_type, iterations
                );
                println!(
                    "Would run {} benchmark with {} iterations",
                    benchmark_type, iterations
                );
            }
        }

        Ok(())
    }

    // Service command implementations

    async fn execute_service_command(&self, cmd: ServiceCommands) -> Result<()> {
        match cmd.action {
            ServiceActions::Start => {
                info!("Starting TilleRS service");
                println!("TilleRS service would be started");
            }
            ServiceActions::Stop => {
                info!("Stopping TilleRS service");
                println!("TilleRS service would be stopped");
            }
            ServiceActions::Restart => {
                info!("Restarting TilleRS service");
                println!("TilleRS service would be restarted");
            }
            ServiceActions::Status => {
                info!("Checking service status");
                println!("Service status would be displayed here");
            }
            ServiceActions::Install => {
                info!("Installing TilleRS as system service");
                println!("TilleRS would be installed as a system service");
            }
            ServiceActions::Uninstall => {
                info!("Uninstalling TilleRS system service");
                println!("TilleRS system service would be uninstalled");
            }
        }

        Ok(())
    }
}

/// Run the CLI interface
pub async fn run_cli(
    workspace_manager: Arc<WorkspaceManager>,
    error_recovery: Arc<ErrorRecoveryManager>,
) -> Result<()> {
    let cli = TilleRSCli::parse();

    // Set up logging level based on verbosity
    if cli.verbose {
        // In a real implementation, this would increase log level
        debug!("Verbose output enabled");
    }

    let executor = TilleRSCliExecutor::new(workspace_manager, error_recovery, cli.json);

    if let Err(e) = executor.execute(cli.command).await {
        if cli.json {
            let error_json = serde_json::json!({
                "error": true,
                "message": e.to_string()
            });
            println!("{}", serde_json::to_string_pretty(&error_json)?);
        } else {
            error!("Command failed: {}", e);
        }
        std::process::exit(1);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_parsing() {
        // Test basic command parsing
        let cli = TilleRSCli::try_parse_from(&["tillers", "workspace", "list"]);
        assert!(cli.is_ok());

        let cli = cli.unwrap();
        match cli.command {
            Commands::Workspace(workspace_cmd) => match workspace_cmd.action {
                WorkspaceActions::List => (),
                _ => panic!("Expected List action"),
            },
            _ => panic!("Expected Workspace command"),
        }
    }

    #[test]
    fn test_global_flags() {
        let cli = TilleRSCli::try_parse_from(&[
            "tillers",
            "--verbose",
            "--json",
            "diagnostics",
            "health",
        ]);
        assert!(cli.is_ok());

        let cli = cli.unwrap();
        assert!(cli.verbose);
        assert!(cli.json);
    }

    #[test]
    fn test_workspace_create_command() {
        let cli = TilleRSCli::try_parse_from(&[
            "tillers",
            "workspace",
            "create",
            "test-workspace",
            "--description",
            "Test workspace",
            "--shortcut",
            "opt+1",
            "--auto-arrange",
        ]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_diagnostics_commands() {
        let health_cli = TilleRSCli::try_parse_from(&["tillers", "diagnostics", "health"]);
        assert!(health_cli.is_ok());

        let benchmark_cli = TilleRSCli::try_parse_from(&[
            "tillers",
            "diagnostics",
            "benchmark",
            "workspace-switching",
            "--iterations",
            "50",
        ]);
        assert!(benchmark_cli.is_ok());
    }
}
