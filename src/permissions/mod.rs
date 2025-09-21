//! macOS Permissions Management for TilleRS

use crate::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{debug, error, info, warn};

/// Errors related to permission checking and management
#[derive(Error, Debug)]
pub enum PermissionError {
    #[error("Permission denied: {permission}")]
    Denied { permission: String },
    #[error("Permission check failed: {message}")]
    CheckFailed { message: String },
    #[error("macOS API error: {message}")]
    SystemError { message: String },
}

/// Types of permissions required by TilleRS
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PermissionType {
    /// Accessibility API access for window manipulation
    Accessibility,
    /// Input monitoring for global keyboard shortcuts
    InputMonitoring,
    /// Screen recording (optional, for advanced window detection)
    ScreenRecording,
}

impl std::fmt::Display for PermissionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PermissionType::Accessibility => write!(f, "Accessibility"),
            PermissionType::InputMonitoring => write!(f, "Input Monitoring"),
            PermissionType::ScreenRecording => write!(f, "Screen Recording"),
        }
    }
}

/// Permission status for a specific permission type
#[derive(Debug, Clone, PartialEq)]
pub enum PermissionStatus {
    /// Permission is granted
    Granted,
    /// Permission is denied
    Denied,
    /// Permission status is unknown or indeterminate
    Unknown,
    /// Permission was not requested yet
    NotRequested,
}

/// Configuration for permission checking behavior
#[derive(Debug, Clone)]
pub struct PermissionConfig {
    /// How often to check permission status (in seconds)
    pub check_interval: Duration,
    /// Whether to automatically prompt for permissions
    pub auto_prompt: bool,
    /// Required permissions (app will not function without these)
    pub required_permissions: Vec<PermissionType>,
    /// Optional permissions (app can function with degraded features)
    pub optional_permissions: Vec<PermissionType>,
}

impl Default for PermissionConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(30),
            auto_prompt: true,
            required_permissions: vec![
                PermissionType::Accessibility,
                PermissionType::InputMonitoring,
            ],
            optional_permissions: vec![PermissionType::ScreenRecording],
        }
    }
}

/// Permission checker and manager for macOS permissions
pub struct PermissionChecker {
    config: PermissionConfig,
    status_cache: HashMap<PermissionType, (PermissionStatus, Instant)>,
    last_check: Option<Instant>,
}

impl PermissionChecker {
    /// Create a new permission checker with the given configuration
    pub fn new(config: PermissionConfig) -> Self {
        Self {
            config,
            status_cache: HashMap::new(),
            last_check: None,
        }
    }

    /// Check all required and optional permissions
    pub async fn check_all_permissions(
        &mut self,
    ) -> Result<HashMap<PermissionType, PermissionStatus>> {
        let mut results = HashMap::new();

        // Check required permissions
        for permission in self.config.required_permissions.clone() {
            let status = self.check_permission(permission.clone()).await?;
            results.insert(permission, status);
        }

        // Check optional permissions
        for permission in self.config.optional_permissions.clone() {
            let status = self.check_permission(permission.clone()).await?;
            results.insert(permission, status);
        }

        self.last_check = Some(Instant::now());
        info!("Permission check completed: {:?}", results);

        Ok(results)
    }

    /// Check a specific permission status
    pub async fn check_permission(
        &mut self,
        permission: PermissionType,
    ) -> Result<PermissionStatus> {
        // Check cache first
        if let Some((status, timestamp)) = self.status_cache.get(&permission) {
            if timestamp.elapsed() < self.config.check_interval {
                debug!(
                    "Using cached permission status for {}: {:?}",
                    permission, status
                );
                return Ok(status.clone());
            }
        }

        let status = match permission {
            PermissionType::Accessibility => self.check_accessibility_permission().await?,
            PermissionType::InputMonitoring => self.check_input_monitoring_permission().await?,
            PermissionType::ScreenRecording => self.check_screen_recording_permission().await?,
        };

        // Update cache
        self.status_cache
            .insert(permission.clone(), (status.clone(), Instant::now()));

        debug!("Permission check for {}: {:?}", permission, status);
        Ok(status)
    }

    /// Check if all required permissions are granted
    pub async fn all_required_permissions_granted(&mut self) -> Result<bool> {
        for permission in self.config.required_permissions.clone() {
            let status = self.check_permission(permission.clone()).await?;
            if status != PermissionStatus::Granted {
                warn!(
                    "Required permission {} not granted: {:?}",
                    permission, status
                );
                return Ok(false);
            }
        }

        info!("All required permissions are granted");
        Ok(true)
    }

    /// Request permissions if auto_prompt is enabled
    pub async fn request_permissions_if_needed(&mut self) -> Result<()> {
        if !self.config.auto_prompt {
            debug!("Auto-prompt disabled, skipping permission requests");
            return Ok(());
        }

        let statuses = self.check_all_permissions().await?;

        for (permission, status) in statuses {
            if self.config.required_permissions.contains(&permission)
                && status != PermissionStatus::Granted
            {
                info!("Requesting required permission: {}", permission);
                self.request_permission(permission).await?;
            }
        }

        Ok(())
    }

    /// Request a specific permission
    pub async fn request_permission(&self, permission: PermissionType) -> Result<()> {
        match permission {
            PermissionType::Accessibility => self.request_accessibility_permission().await,
            PermissionType::InputMonitoring => self.request_input_monitoring_permission().await,
            PermissionType::ScreenRecording => self.request_screen_recording_permission().await,
        }
    }

    /// Get user-friendly instructions for enabling a permission
    pub fn get_permission_instructions(&self, permission: &PermissionType) -> String {
        match permission {
            PermissionType::Accessibility => "To enable TilleRS window management:\n\
                1. Open System Preferences > Security & Privacy > Privacy\n\
                2. Select 'Accessibility' from the list\n\
                3. Click the lock icon and enter your password\n\
                4. Check the box next to 'TilleRS'\n\
                5. Restart TilleRS"
                .to_string(),
            PermissionType::InputMonitoring => "To enable TilleRS keyboard shortcuts:\n\
                1. Open System Preferences > Security & Privacy > Privacy\n\
                2. Select 'Input Monitoring' from the list\n\
                3. Click the lock icon and enter your password\n\
                4. Check the box next to 'TilleRS'\n\
                5. Restart TilleRS"
                .to_string(),
            PermissionType::ScreenRecording => "To enable advanced window detection (optional):\n\
                1. Open System Preferences > Security & Privacy > Privacy\n\
                2. Select 'Screen Recording' from the list\n\
                3. Click the lock icon and enter your password\n\
                4. Check the box next to 'TilleRS'\n\
                5. Restart TilleRS"
                .to_string(),
        }
    }

    /// Get a summary of current permission status
    pub async fn get_permission_summary(&mut self) -> Result<PermissionSummary> {
        let statuses = self.check_all_permissions().await?;

        let mut required_granted = 0;
        let required_total = self.config.required_permissions.len();
        let mut optional_granted = 0;
        let optional_total = self.config.optional_permissions.len();

        for (permission, status) in &statuses {
            if status == &PermissionStatus::Granted {
                if self.config.required_permissions.contains(permission) {
                    required_granted += 1;
                } else {
                    optional_granted += 1;
                }
            }
        }

        let all_required_granted = required_granted == required_total;
        let can_function = all_required_granted;

        Ok(PermissionSummary {
            all_required_granted,
            can_function,
            required_granted,
            required_total,
            optional_granted,
            optional_total,
            statuses,
        })
    }

    // Platform-specific permission checking methods

    async fn check_accessibility_permission(&self) -> Result<PermissionStatus> {
        debug!("Checking accessibility permission");

        let granted = {
            #[cfg(target_os = "macos")]
            {
                crate::macos::permissions::is_accessibility_permission_granted()?
            }

            #[cfg(not(target_os = "macos"))]
            {
                self.simulate_permission_check("accessibility")
            }
        };

        Ok(if granted {
            PermissionStatus::Granted
        } else {
            PermissionStatus::Denied
        })
    }

    async fn check_input_monitoring_permission(&self) -> Result<PermissionStatus> {
        debug!("Checking input monitoring permission");

        let granted = {
            #[cfg(target_os = "macos")]
            {
                crate::macos::permissions::is_input_monitoring_permission_granted()?
            }

            #[cfg(not(target_os = "macos"))]
            {
                self.simulate_permission_check("input_monitoring")
            }
        };

        Ok(if granted {
            PermissionStatus::Granted
        } else {
            PermissionStatus::Denied
        })
    }

    async fn check_screen_recording_permission(&self) -> Result<PermissionStatus> {
        debug!("Checking screen recording permission");

        let granted = {
            #[cfg(target_os = "macos")]
            {
                crate::macos::permissions::is_screen_recording_permission_granted()?
            }

            #[cfg(not(target_os = "macos"))]
            {
                self.simulate_permission_check("screen_recording")
            }
        };

        Ok(if granted {
            PermissionStatus::Granted
        } else {
            PermissionStatus::Denied
        })
    }

    async fn request_accessibility_permission(&self) -> Result<()> {
        info!("Requesting accessibility permission");

        #[cfg(target_os = "macos")]
        {
            let granted = crate::macos::permissions::prompt_accessibility_permission()?;
            if !granted {
                warn!("Accessibility permission not yet granted after prompt");
                if let Err(err) = crate::macos::permissions::open_privacy_pane(
                    crate::macos::permissions::PrivacyPane::Accessibility,
                ) {
                    error!("Failed to open Accessibility privacy pane: {err}");
                }
            }
            Ok(())
        }

        #[cfg(not(target_os = "macos"))]
        {
            println!("Please grant Accessibility permission in System Preferences");
            Ok(())
        }
    }

    async fn request_input_monitoring_permission(&self) -> Result<()> {
        info!("Requesting input monitoring permission");

        #[cfg(target_os = "macos")]
        {
            match crate::macos::permissions::prompt_input_monitoring_permission() {
                Ok(true) => (),
                Ok(false) => {
                    warn!("Input Monitoring permission denied");
                    if let Err(err) = crate::macos::permissions::open_privacy_pane(
                        crate::macos::permissions::PrivacyPane::InputMonitoring,
                    ) {
                        error!("Failed to open Input Monitoring privacy pane: {err}");
                    }
                }
                Err(err) => {
                    error!("Failed to request Input Monitoring access: {err}");
                    crate::macos::permissions::open_privacy_pane(
                        crate::macos::permissions::PrivacyPane::InputMonitoring,
                    )
                    .ok();
                }
            }
            Ok(())
        }

        #[cfg(not(target_os = "macos"))]
        {
            println!("Please grant Input Monitoring permission in System Preferences");
            Ok(())
        }
    }

    async fn request_screen_recording_permission(&self) -> Result<()> {
        info!("Requesting screen recording permission");

        #[cfg(target_os = "macos")]
        {
            match crate::macos::permissions::prompt_screen_recording_permission() {
                Ok(true) => (),
                Ok(false) => {
                    warn!("Screen Recording permission denied");
                    if let Err(err) = crate::macos::permissions::open_privacy_pane(
                        crate::macos::permissions::PrivacyPane::ScreenRecording,
                    ) {
                        error!("Failed to open Screen Recording privacy pane: {err}");
                    }
                }
                Err(err) => {
                    error!("Failed to request Screen Recording access: {err}");
                    crate::macos::permissions::open_privacy_pane(
                        crate::macos::permissions::PrivacyPane::ScreenRecording,
                    )
                    .ok();
                }
            }
            Ok(())
        }

        #[cfg(not(target_os = "macos"))]
        {
            println!("Please grant Screen Recording permission in System Preferences");
            Ok(())
        }
    }

    // Simulation method for testing - would be replaced with actual macOS API calls
    #[cfg(not(target_os = "macos"))]
    fn simulate_permission_check(&self, permission_name: &str) -> bool {
        // For simulation purposes, check if environment variable is set
        let env_var = format!("TILLERS_PERMISSION_{}", permission_name.to_uppercase());
        std::env::var(env_var).unwrap_or_else(|_| "false".to_string()) == "true"
    }
}

/// Summary of current permission status
#[derive(Debug, Clone)]
pub struct PermissionSummary {
    /// Whether all required permissions are granted
    pub all_required_granted: bool,
    /// Whether the app can function (has minimum required permissions)
    pub can_function: bool,
    /// Number of required permissions granted
    pub required_granted: usize,
    /// Total number of required permissions
    pub required_total: usize,
    /// Number of optional permissions granted
    pub optional_granted: usize,
    /// Total number of optional permissions
    pub optional_total: usize,
    /// Individual permission statuses
    pub statuses: HashMap<PermissionType, PermissionStatus>,
}

impl PermissionSummary {
    /// Get a human-readable description of the permission status
    pub fn description(&self) -> String {
        if self.all_required_granted {
            format!(
                "All permissions granted ({}/{} required, {}/{} optional)",
                self.required_granted,
                self.required_total,
                self.optional_granted,
                self.optional_total
            )
        } else {
            format!(
                "Missing permissions ({}/{} required granted)",
                self.required_granted, self.required_total
            )
        }
    }

    /// Get a list of missing required permissions
    pub fn missing_required_permissions(&self) -> Vec<PermissionType> {
        self.statuses
            .iter()
            .filter_map(|(permission, status)| {
                if status != &PermissionStatus::Granted {
                    Some(permission.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn test_permission_checker_creation() {
        let config = PermissionConfig::default();
        let checker = PermissionChecker::new(config);
        assert!(checker.status_cache.is_empty());
    }

    #[tokio::test]
    async fn test_permission_check_with_env() {
        env::set_var("TILLERS_PERMISSION_ACCESSIBILITY", "true");
        env::set_var("TILLERS_PERMISSION_INPUT_MONITORING", "false");

        let config = PermissionConfig::default();
        let mut checker = PermissionChecker::new(config);

        let accessibility_status = checker
            .check_permission(PermissionType::Accessibility)
            .await
            .unwrap();
        assert_eq!(accessibility_status, PermissionStatus::Granted);

        let input_status = checker
            .check_permission(PermissionType::InputMonitoring)
            .await
            .unwrap();
        assert_eq!(input_status, PermissionStatus::Denied);

        // Clean up
        env::remove_var("TILLERS_PERMISSION_ACCESSIBILITY");
        env::remove_var("TILLERS_PERMISSION_INPUT_MONITORING");
    }

    #[tokio::test]
    async fn test_all_required_permissions_check() {
        env::set_var("TILLERS_PERMISSION_ACCESSIBILITY", "true");
        env::set_var("TILLERS_PERMISSION_INPUT_MONITORING", "true");

        let config = PermissionConfig::default();
        let mut checker = PermissionChecker::new(config);

        let all_granted = checker.all_required_permissions_granted().await.unwrap();
        assert!(all_granted);

        // Clean up
        env::remove_var("TILLERS_PERMISSION_ACCESSIBILITY");
        env::remove_var("TILLERS_PERMISSION_INPUT_MONITORING");
    }

    #[tokio::test]
    async fn test_permission_summary() {
        env::set_var("TILLERS_PERMISSION_ACCESSIBILITY", "true");
        env::set_var("TILLERS_PERMISSION_INPUT_MONITORING", "false");

        let config = PermissionConfig::default();
        let mut checker = PermissionChecker::new(config);

        let summary = checker.get_permission_summary().await.unwrap();
        assert!(!summary.all_required_granted);
        assert!(!summary.can_function);
        assert_eq!(summary.required_granted, 1);
        assert_eq!(summary.required_total, 2);

        // Clean up
        env::remove_var("TILLERS_PERMISSION_ACCESSIBILITY");
        env::remove_var("TILLERS_PERMISSION_INPUT_MONITORING");
    }

    #[test]
    fn test_permission_instructions() {
        let config = PermissionConfig::default();
        let checker = PermissionChecker::new(config);

        let instructions = checker.get_permission_instructions(&PermissionType::Accessibility);
        assert!(instructions.contains("Accessibility"));
        assert!(instructions.contains("System Preferences"));
    }
}
