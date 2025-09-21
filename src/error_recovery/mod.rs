//! Error recovery and resilience for TilleRS API failures
//!
//! This module provides comprehensive error recovery strategies for macOS API failures,
//! permission issues, and system-level problems that can occur during window management.

use crate::permissions::{PermissionChecker, PermissionType};
use crate::{Result, TilleRSError};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Configuration for error recovery behavior
#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    /// Maximum number of retry attempts for transient failures
    pub max_retries: usize,
    /// Base delay between retry attempts
    pub base_retry_delay: Duration,
    /// Maximum delay between retry attempts (for exponential backoff)
    pub max_retry_delay: Duration,
    /// Timeout for individual API calls
    pub api_timeout: Duration,
    /// Interval for checking permission status
    pub permission_check_interval: Duration,
    /// Whether to automatically attempt permission recovery
    pub auto_permission_recovery: bool,
    /// Circuit breaker threshold (failures before disabling)
    pub circuit_breaker_threshold: usize,
    /// Circuit breaker recovery time
    pub circuit_breaker_recovery_time: Duration,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_retry_delay: Duration::from_millis(100),
            max_retry_delay: Duration::from_secs(5),
            api_timeout: Duration::from_secs(2),
            permission_check_interval: Duration::from_secs(30),
            auto_permission_recovery: true,
            circuit_breaker_threshold: 5,
            circuit_breaker_recovery_time: Duration::from_secs(60),
        }
    }
}

/// Types of recoverable errors
#[derive(Debug, Clone, PartialEq)]
pub enum RecoverableError {
    /// Permission denied - can be recovered by requesting permission
    PermissionDenied(PermissionType),
    /// API temporarily unavailable - can be recovered with retry
    ApiUnavailable,
    /// Window not found - might be recoverable if window appears later
    WindowNotFound(u32),
    /// Workspace operation failed - can be retried
    WorkspaceOperationFailed,
    /// System overloaded - should back off and retry
    SystemOverloaded,
    /// Network or IPC failure - can be retried
    CommunicationFailure,
}

/// Recovery strategy for different error types
#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    /// Retry with exponential backoff
    RetryWithBackoff { attempts: usize, delay: Duration },
    /// Request missing permissions
    RequestPermissions { permissions: Vec<PermissionType> },
    /// Wait and retry once
    WaitAndRetry { wait_time: Duration },
    /// Circuit breaker - temporarily disable functionality
    CircuitBreaker { until: Instant },
    /// No recovery possible
    NoRecovery,
}

/// Circuit breaker state for API endpoints
#[derive(Debug, Clone)]
struct CircuitBreakerState {
    failures: usize,
    last_failure: Option<Instant>,
    state: CircuitState,
}

#[derive(Debug, Clone, PartialEq)]
enum CircuitState {
    Closed, // Normal operation
    Open,   // Failing, blocking requests
}

/// Error recovery manager for TilleRS
pub struct ErrorRecoveryManager {
    config: RecoveryConfig,
    permission_checker: Arc<RwLock<PermissionChecker>>,
    circuit_breakers: Arc<RwLock<HashMap<String, CircuitBreakerState>>>,
    last_permission_check: Arc<RwLock<Option<Instant>>>,
}

impl ErrorRecoveryManager {
    /// Create a new error recovery manager
    pub fn new(config: RecoveryConfig, permission_checker: PermissionChecker) -> Self {
        Self {
            config,
            permission_checker: Arc::new(RwLock::new(permission_checker)),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            last_permission_check: Arc::new(RwLock::new(None)),
        }
    }

    /// Attempt to recover from an error and retry the operation
    pub async fn recover_and_retry<F, T>(&self, operation_name: &str, mut operation: F) -> Result<T>
    where
        F: FnMut() -> Result<T> + Send,
    {
        // Check circuit breaker state
        if self.is_circuit_open(operation_name).await {
            return Err(TilleRSError::MacOSAPIError(format!(
                "Circuit breaker open for {}",
                operation_name
            ))
            .into());
        }

        let mut attempt = 0;
        let mut last_error = None;

        while attempt <= self.config.max_retries {
            // Add timeout to the operation
            let result = tokio::time::timeout(self.config.api_timeout, async { operation() }).await;

            match result {
                Ok(Ok(value)) => {
                    // Success - reset circuit breaker
                    self.reset_circuit_breaker(operation_name).await;
                    return Ok(value);
                }
                Ok(Err(error)) => {
                    last_error = Some(error);
                    let recoverable_error = self.classify_error(last_error.as_ref().unwrap()).await;

                    if let Some(recoverable) = recoverable_error {
                        let strategy = self
                            .determine_recovery_strategy(&recoverable, attempt)
                            .await;

                        match strategy {
                            RecoveryStrategy::RetryWithBackoff { delay, .. } => {
                                if attempt < self.config.max_retries {
                                    warn!(
                                        "Operation {} failed (attempt {}), retrying in {:?}: {:?}",
                                        operation_name,
                                        attempt + 1,
                                        delay,
                                        recoverable
                                    );
                                    tokio::time::sleep(delay).await;
                                    attempt += 1;
                                    continue;
                                }
                            }
                            RecoveryStrategy::RequestPermissions { permissions } => {
                                if self.config.auto_permission_recovery {
                                    info!("Attempting permission recovery for {:?}", permissions);
                                    if self.recover_permissions(&permissions).await.is_ok() {
                                        attempt += 1;
                                        continue;
                                    }
                                }
                            }
                            RecoveryStrategy::WaitAndRetry { wait_time } => {
                                warn!("Waiting {:?} before retrying {}", wait_time, operation_name);
                                tokio::time::sleep(wait_time).await;
                                attempt += 1;
                                continue;
                            }
                            RecoveryStrategy::CircuitBreaker { until } => {
                                self.open_circuit_breaker(operation_name, until).await;
                                return Err(TilleRSError::MacOSAPIError(format!(
                                    "Circuit breaker activated for {}",
                                    operation_name
                                ))
                                .into());
                            }
                            RecoveryStrategy::NoRecovery => {
                                break;
                            }
                        }
                    }
                }
                Err(_timeout) => {
                    warn!(
                        "Operation {} timed out (attempt {})",
                        operation_name,
                        attempt + 1
                    );
                    last_error = Some(
                        TilleRSError::MacOSAPIError(format!(
                            "Operation {} timed out",
                            operation_name
                        ))
                        .into(),
                    );
                }
            }

            // Record failure for circuit breaker
            self.record_failure(operation_name).await;
            attempt += 1;

            // Apply exponential backoff
            if attempt <= self.config.max_retries {
                let delay = self.calculate_backoff_delay(attempt);
                tokio::time::sleep(delay).await;
            }
        }

        // All retries exhausted
        error!(
            "Operation {} failed after {} attempts",
            operation_name,
            self.config.max_retries + 1
        );
        Err(last_error.unwrap_or_else(|| {
            TilleRSError::MacOSAPIError(format!(
                "Operation {} failed after all retries",
                operation_name
            ))
            .into()
        }))
    }

    /// Check and recover permissions if needed
    pub async fn check_and_recover_permissions(&self) -> Result<bool> {
        let mut last_check = self.last_permission_check.write().await;

        // Check if we need to refresh permission status
        let should_check = match *last_check {
            Some(last) => last.elapsed() >= self.config.permission_check_interval,
            None => true,
        };

        if !should_check {
            debug!("Permission check skipped - too recent");
            return Ok(true);
        }

        *last_check = Some(Instant::now());
        drop(last_check);

        let mut checker = self.permission_checker.write().await;

        // Check if all required permissions are granted
        let all_granted = checker.all_required_permissions_granted().await?;

        if !all_granted {
            warn!("Required permissions not granted");

            if self.config.auto_permission_recovery {
                info!("Attempting automatic permission recovery");
                checker.request_permissions_if_needed().await?;

                // Re-check after permission request
                tokio::time::sleep(Duration::from_millis(500)).await;
                return checker.all_required_permissions_granted().await;
            }
        }

        Ok(all_granted)
    }

    /// Get permission instructions for manual recovery
    pub async fn get_permission_recovery_instructions(&self) -> Result<Vec<String>> {
        let mut checker = self.permission_checker.write().await;
        let summary = checker.get_permission_summary().await?;

        let mut instructions = Vec::new();

        for permission_type in summary.missing_required_permissions() {
            let instruction = checker.get_permission_instructions(&permission_type);
            instructions.push(instruction);
        }

        Ok(instructions)
    }

    /// Get current system health status
    pub async fn get_health_status(&self) -> Result<HealthStatus> {
        let permission_ok = self.check_and_recover_permissions().await?;

        let circuit_breakers = self.circuit_breakers.read().await;
        let active_breakers: Vec<String> = circuit_breakers
            .iter()
            .filter(|(_, state)| state.state != CircuitState::Closed)
            .map(|(name, _)| name.clone())
            .collect();

        Ok(HealthStatus {
            permissions_granted: permission_ok,
            active_circuit_breakers: active_breakers,
            last_permission_check: *self.last_permission_check.read().await,
        })
    }

    // Private helper methods

    async fn classify_error(&self, error: &anyhow::Error) -> Option<RecoverableError> {
        if let Some(tillers_error) = error.downcast_ref::<TilleRSError>() {
            match tillers_error {
                TilleRSError::PermissionDenied(msg) => {
                    // Try to map permission message to permission type
                    if msg.contains("Accessibility") || msg.contains("accessibility") {
                        Some(RecoverableError::PermissionDenied(
                            PermissionType::Accessibility,
                        ))
                    } else if msg.contains("Input") || msg.contains("input") {
                        Some(RecoverableError::PermissionDenied(
                            PermissionType::InputMonitoring,
                        ))
                    } else if msg.contains("Screen") || msg.contains("screen") {
                        Some(RecoverableError::PermissionDenied(
                            PermissionType::ScreenRecording,
                        ))
                    } else {
                        Some(RecoverableError::PermissionDenied(
                            PermissionType::Accessibility,
                        ))
                    }
                }
                TilleRSError::WindowNotFound(window_id) => {
                    Some(RecoverableError::WindowNotFound(*window_id))
                }
                TilleRSError::MacOSAPIError(msg) => {
                    if msg.contains("timeout") || msg.contains("unavailable") {
                        Some(RecoverableError::ApiUnavailable)
                    } else if msg.contains("overload") || msg.contains("busy") {
                        Some(RecoverableError::SystemOverloaded)
                    } else {
                        Some(RecoverableError::CommunicationFailure)
                    }
                }
                TilleRSError::WorkspaceNotFound(_) => {
                    Some(RecoverableError::WorkspaceOperationFailed)
                }
                _ => None,
            }
        } else {
            // Generic error classification
            let error_msg = error.to_string().to_lowercase();
            if error_msg.contains("permission") {
                Some(RecoverableError::PermissionDenied(
                    PermissionType::Accessibility,
                ))
            } else if error_msg.contains("timeout") {
                Some(RecoverableError::ApiUnavailable)
            } else {
                Some(RecoverableError::CommunicationFailure)
            }
        }
    }

    async fn determine_recovery_strategy(
        &self,
        error: &RecoverableError,
        attempt: usize,
    ) -> RecoveryStrategy {
        match error {
            RecoverableError::PermissionDenied(permission_type) => {
                RecoveryStrategy::RequestPermissions {
                    permissions: vec![permission_type.clone()],
                }
            }
            RecoverableError::ApiUnavailable | RecoverableError::CommunicationFailure => {
                if attempt >= self.config.circuit_breaker_threshold {
                    RecoveryStrategy::CircuitBreaker {
                        until: Instant::now() + self.config.circuit_breaker_recovery_time,
                    }
                } else {
                    RecoveryStrategy::RetryWithBackoff {
                        attempts: attempt,
                        delay: self.calculate_backoff_delay(attempt),
                    }
                }
            }
            RecoverableError::WindowNotFound(_) => RecoveryStrategy::WaitAndRetry {
                wait_time: Duration::from_millis(200),
            },
            RecoverableError::WorkspaceOperationFailed => RecoveryStrategy::RetryWithBackoff {
                attempts: attempt,
                delay: self.calculate_backoff_delay(attempt),
            },
            RecoverableError::SystemOverloaded => RecoveryStrategy::WaitAndRetry {
                wait_time: Duration::from_secs(1),
            },
        }
    }

    async fn recover_permissions(&self, permissions: &[PermissionType]) -> Result<()> {
        let checker = self.permission_checker.write().await;

        for permission in permissions {
            info!("Requesting permission: {:?}", permission);
            checker.request_permission(permission.clone()).await?;
        }

        Ok(())
    }

    fn calculate_backoff_delay(&self, attempt: usize) -> Duration {
        let delay = self.config.base_retry_delay * (2_u32.pow(attempt as u32));
        std::cmp::min(delay, self.config.max_retry_delay)
    }

    async fn is_circuit_open(&self, operation_name: &str) -> bool {
        let breakers = self.circuit_breakers.read().await;

        if let Some(state) = breakers.get(operation_name) {
            match state.state {
                CircuitState::Open => {
                    if let Some(last_failure) = state.last_failure {
                        last_failure.elapsed() < self.config.circuit_breaker_recovery_time
                    } else {
                        false
                    }
                }
                CircuitState::Closed => false,
            }
        } else {
            false
        }
    }

    async fn record_failure(&self, operation_name: &str) {
        let mut breakers = self.circuit_breakers.write().await;

        let state = breakers
            .entry(operation_name.to_string())
            .or_insert_with(|| CircuitBreakerState {
                failures: 0,
                last_failure: None,
                state: CircuitState::Closed,
            });

        state.failures += 1;
        state.last_failure = Some(Instant::now());

        if state.failures >= self.config.circuit_breaker_threshold {
            state.state = CircuitState::Open;
            warn!(
                "Circuit breaker opened for {} after {} failures",
                operation_name, state.failures
            );
        }
    }

    async fn open_circuit_breaker(&self, operation_name: &str, until: Instant) {
        let mut breakers = self.circuit_breakers.write().await;

        let state = breakers
            .entry(operation_name.to_string())
            .or_insert_with(|| CircuitBreakerState {
                failures: 0,
                last_failure: None,
                state: CircuitState::Closed,
            });

        state.state = CircuitState::Open;
        state.last_failure = Some(until);
    }

    async fn reset_circuit_breaker(&self, operation_name: &str) {
        let mut breakers = self.circuit_breakers.write().await;

        if let Some(state) = breakers.get_mut(operation_name) {
            if state.failures > 0 {
                debug!(
                    "Resetting circuit breaker for {} after successful operation",
                    operation_name
                );
                state.failures = 0;
                state.state = CircuitState::Closed;
                state.last_failure = None;
            }
        }
    }
}

/// System health status information
#[derive(Debug, Clone)]
pub struct HealthStatus {
    /// Whether all required permissions are granted
    pub permissions_granted: bool,
    /// Names of operations with active circuit breakers
    pub active_circuit_breakers: Vec<String>,
    /// Last time permissions were checked
    pub last_permission_check: Option<Instant>,
}

impl HealthStatus {
    /// Check if the system is healthy and ready for operation
    pub fn is_healthy(&self) -> bool {
        self.permissions_granted && self.active_circuit_breakers.is_empty()
    }

    /// Get a human-readable description of the system status
    pub fn description(&self) -> String {
        if self.is_healthy() {
            return "System healthy - all permissions granted, no circuit breakers active"
                .to_string();
        }

        let mut issues: Vec<String> = Vec::new();

        if !self.permissions_granted {
            issues.push("Missing required permissions".to_string());
        }

        if !self.active_circuit_breakers.is_empty() {
            issues.push(format!(
                "Circuit breakers active: {}",
                self.active_circuit_breakers.join(", ")
            ));
        }

        if issues.is_empty() {
            "System health degraded".to_string()
        } else {
            format!("System issues: {}", issues.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::PermissionConfig;

    #[tokio::test]
    async fn test_recovery_manager_creation() {
        let config = RecoveryConfig::default();
        let permission_checker = PermissionChecker::new(PermissionConfig::default());
        let manager = ErrorRecoveryManager::new(config, permission_checker);

        let health = manager.get_health_status().await.unwrap();
        assert!(health.active_circuit_breakers.is_empty());
    }

    #[tokio::test]
    async fn test_error_classification() {
        let config = RecoveryConfig::default();
        let permission_checker = PermissionChecker::new(PermissionConfig::default());
        let manager = ErrorRecoveryManager::new(config, permission_checker);

        let permission_error =
            TilleRSError::PermissionDenied("Accessibility permission required".to_string());
        let classified = manager.classify_error(&permission_error.into()).await;

        match classified {
            Some(RecoverableError::PermissionDenied(PermissionType::Accessibility)) => (),
            _ => panic!("Expected accessibility permission error"),
        }
    }

    #[tokio::test]
    async fn test_backoff_calculation() {
        let config = RecoveryConfig::default();
        let permission_checker = PermissionChecker::new(PermissionConfig::default());
        let manager = ErrorRecoveryManager::new(config, permission_checker);

        let delay1 = manager.calculate_backoff_delay(0);
        let delay2 = manager.calculate_backoff_delay(1);
        let delay3 = manager.calculate_backoff_delay(2);

        assert!(delay2 > delay1);
        assert!(delay3 > delay2);
        assert!(delay3 <= manager.config.max_retry_delay);
    }

    #[test]
    fn test_health_status() {
        let healthy_status = HealthStatus {
            permissions_granted: true,
            active_circuit_breakers: vec![],
            last_permission_check: Some(Instant::now()),
        };

        assert!(healthy_status.is_healthy());
        assert!(healthy_status.description().contains("healthy"));

        let unhealthy_status = HealthStatus {
            permissions_granted: false,
            active_circuit_breakers: vec!["window_manager".to_string()],
            last_permission_check: Some(Instant::now()),
        };

        assert!(!unhealthy_status.is_healthy());
        assert!(unhealthy_status.description().contains("issues"));
    }
}
