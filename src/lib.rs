//! TilleRS - Keyboard-First Tiling Window Manager for macOS
//!
//! TilleRS automatically organizes windows into logical workspaces, enabling instant
//! context switching between projects while maintaining predictable window layouts
//! across multiple monitors.

pub mod config;
pub mod macos;
pub mod models;
pub mod services;

pub use models::*;
pub use services::*;

/// Result type alias for TilleRS operations
pub type Result<T> = anyhow::Result<T>;

/// Error types specific to TilleRS operations
#[derive(thiserror::Error, Debug)]
pub enum TilleRSError {
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Window not found: {0}")]
    WindowNotFound(u32),

    #[error("Workspace not found: {0}")]
    WorkspaceNotFound(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("macOS API error: {0}")]
    MacOSAPIError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),
}
