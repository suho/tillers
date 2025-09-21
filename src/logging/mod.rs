//! Structured logging configuration for TilleRS

use std::str::FromStr;
use tracing::{info, warn};
use tracing_subscriber::{
    fmt::{self, time::UtcTime},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

/// Logging configuration for TilleRS
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: LogLevel,
    /// Log format (json, pretty, compact)
    pub format: LogFormat,
    /// Output destination (stdout, file)
    pub output: LogOutput,
    /// File path for file output
    pub file_path: Option<String>,
    /// Include source file and line numbers
    pub include_source: bool,
    /// Include thread names
    pub include_thread_names: bool,
    /// Performance tracing enabled
    pub performance_tracing: bool,
}

/// Log levels supported by TilleRS
#[derive(Debug, Clone, PartialEq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(format!("Invalid log level: {}", s)),
        }
    }
}

impl From<LogLevel> for tracing::Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => tracing::Level::TRACE,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Error => tracing::Level::ERROR,
        }
    }
}

/// Log output formats
#[derive(Debug, Clone, PartialEq)]
pub enum LogFormat {
    /// Human-readable format for development
    Pretty,
    /// Compact format for production
    Compact,
    /// JSON format for log aggregation
    Json,
}

impl FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pretty" => Ok(LogFormat::Pretty),
            "compact" => Ok(LogFormat::Compact),
            "json" => Ok(LogFormat::Json),
            _ => Err(format!("Invalid log format: {}", s)),
        }
    }
}

/// Log output destinations
#[derive(Debug, Clone, PartialEq)]
pub enum LogOutput {
    /// Standard output
    Stdout,
    /// File output
    File,
    /// Both stdout and file
    Both,
}

impl FromStr for LogOutput {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "stdout" => Ok(LogOutput::Stdout),
            "file" => Ok(LogOutput::File),
            "both" => Ok(LogOutput::Both),
            _ => Err(format!("Invalid log output: {}", s)),
        }
    }
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: LogFormat::Pretty,
            output: LogOutput::Stdout,
            file_path: None,
            include_source: false,
            include_thread_names: true,
            performance_tracing: false,
        }
    }
}

impl LogConfig {
    /// Create a development-friendly log configuration
    pub fn development() -> Self {
        Self {
            level: LogLevel::Debug,
            format: LogFormat::Pretty,
            output: LogOutput::Stdout,
            file_path: None,
            include_source: true,
            include_thread_names: true,
            performance_tracing: true,
        }
    }

    /// Create a production log configuration
    pub fn production() -> Self {
        Self {
            level: LogLevel::Warn,
            format: LogFormat::Json,
            output: LogOutput::File,
            file_path: Some("/var/log/tillers.log".to_string()),
            include_source: false,
            include_thread_names: false,
            performance_tracing: false,
        }
    }

    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(level) = std::env::var("TILLERS_LOG_LEVEL") {
            if let Ok(parsed_level) = LogLevel::from_str(&level) {
                config.level = parsed_level;
            }
        }

        if let Ok(format) = std::env::var("TILLERS_LOG_FORMAT") {
            if let Ok(parsed_format) = LogFormat::from_str(&format) {
                config.format = parsed_format;
            }
        }

        if let Ok(output) = std::env::var("TILLERS_LOG_OUTPUT") {
            if let Ok(parsed_output) = LogOutput::from_str(&output) {
                config.output = parsed_output;
            }
        }

        if let Ok(file_path) = std::env::var("TILLERS_LOG_FILE") {
            config.file_path = Some(file_path);
        }

        if let Ok(include_source) = std::env::var("TILLERS_LOG_SOURCE") {
            config.include_source = include_source.to_lowercase() == "true";
        }

        if let Ok(performance) = std::env::var("TILLERS_LOG_PERFORMANCE") {
            config.performance_tracing = performance.to_lowercase() == "true";
        }

        config
    }
}

/// Initialize the global tracing subscriber with the given configuration
pub fn init_logging(config: &LogConfig) -> Result<(), Box<dyn std::error::Error>> {
    let filter = create_filter(config)?;

    match (&config.output, &config.file_path) {
        (LogOutput::Stdout, _) => {
            let layer = create_stdout_layer(config);
            tracing_subscriber::registry()
                .with(layer.with_filter(filter))
                .init();
        }
        (LogOutput::File, Some(file_path)) => {
            let layer = create_file_layer(config, file_path)?;
            tracing_subscriber::registry()
                .with(layer.with_filter(filter))
                .init();
        }
        (LogOutput::Both, Some(file_path)) => {
            // For simplicity, default to stdout when both is requested
            // TODO: Implement proper multi-output logging
            let layer = create_stdout_layer(config);
            tracing_subscriber::registry()
                .with(layer.with_filter(filter))
                .init();
            
            warn!("Both stdout and file logging requested, defaulting to stdout. File path: {}", file_path);
        }
        (LogOutput::File, None) | (LogOutput::Both, None) => {
            return Err("File path required for file output".into());
        }
    }

    info!("Logging initialized with config: {:?}", config);
    Ok(())
}

/// Create an environment filter based on the configuration
fn create_filter(config: &LogConfig) -> Result<EnvFilter, Box<dyn std::error::Error>> {
    let base_level = match config.level {
        LogLevel::Trace => "trace",
        LogLevel::Debug => "debug", 
        LogLevel::Info => "info",
        LogLevel::Warn => "warn",
        LogLevel::Error => "error",
    };

    // Start with the base level for our crate
    let mut filter_str = format!("tillers={}", base_level);
    
    // Add performance tracing for specific modules if enabled
    if config.performance_tracing {
        filter_str.push_str(",tillers::services::workspace_manager=trace");
        filter_str.push_str(",tillers::services::window_manager=trace");
        filter_str.push_str(",tillers::services::tiling_engine=trace");
    }

    // Allow environment override
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(filter_str));

    Ok(filter)
}

/// Create a stdout logging layer
fn create_stdout_layer(config: &LogConfig) -> Box<dyn Layer<tracing_subscriber::Registry> + Send + Sync> {
    match config.format {
        LogFormat::Pretty => {
            let layer = fmt::layer()
                .pretty()
                .with_timer(UtcTime::rfc_3339())
                .with_thread_names(config.include_thread_names)
                .with_file(config.include_source)
                .with_line_number(config.include_source);
            
            Box::new(layer)
        }
        LogFormat::Compact => {
            let layer = fmt::layer()
                .compact()
                .with_timer(UtcTime::rfc_3339())
                .with_thread_names(config.include_thread_names)
                .with_file(config.include_source)
                .with_line_number(config.include_source);
            
            Box::new(layer)
        }
        LogFormat::Json => {
            let layer = fmt::layer()
                .json()
                .with_timer(UtcTime::rfc_3339())
                .with_thread_names(config.include_thread_names)
                .with_file(config.include_source)
                .with_line_number(config.include_source);
            
            Box::new(layer)
        }
    }
}

/// Create a file logging layer
fn create_file_layer(config: &LogConfig, file_path: &str) -> Result<Box<dyn Layer<tracing_subscriber::Registry> + Send + Sync>, Box<dyn std::error::Error>> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)?;

    match config.format {
        LogFormat::Pretty => {
            let layer = fmt::layer()
                .pretty()
                .with_writer(file)
                .with_timer(UtcTime::rfc_3339())
                .with_thread_names(config.include_thread_names)
                .with_file(config.include_source)
                .with_line_number(config.include_source);
            
            Ok(Box::new(layer))
        }
        LogFormat::Compact => {
            let layer = fmt::layer()
                .compact()
                .with_writer(file)
                .with_timer(UtcTime::rfc_3339())
                .with_thread_names(config.include_thread_names)
                .with_file(config.include_source)
                .with_line_number(config.include_source);
            
            Ok(Box::new(layer))
        }
        LogFormat::Json => {
            let layer = fmt::layer()
                .json()
                .with_writer(file)
                .with_timer(UtcTime::rfc_3339())
                .with_thread_names(config.include_thread_names)
                .with_file(config.include_source)
                .with_line_number(config.include_source);
            
            Ok(Box::new(layer))
        }
    }
}

/// Performance tracing macros for critical operations
#[macro_export]
macro_rules! trace_performance {
    ($name:expr, $block:block) => {
        {
            let span = tracing::info_span!("performance", operation = $name);
            let _enter = span.enter();
            let start = std::time::Instant::now();
            
            let result = $block;
            
            let duration = start.elapsed();
            tracing::info!(
                operation = $name,
                duration_ms = duration.as_millis(),
                "Performance trace"
            );
            
            result
        }
    };
}

/// Initialize logging for testing environments
#[cfg(test)]
pub fn init_test_logging() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    
    INIT.call_once(|| {
        let config = LogConfig {
            level: LogLevel::Debug,
            format: LogFormat::Compact,
            output: LogOutput::Stdout,
            file_path: None,
            include_source: true,
            include_thread_names: false,
            performance_tracing: true,
        };
        
        if let Err(e) = init_logging(&config) {
            eprintln!("Failed to initialize test logging: {}", e);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_parsing() {
        assert_eq!(LogLevel::from_str("info").unwrap(), LogLevel::Info);
        assert_eq!(LogLevel::from_str("DEBUG").unwrap(), LogLevel::Debug);
        assert!(LogLevel::from_str("invalid").is_err());
    }

    #[test]
    fn test_log_format_parsing() {
        assert_eq!(LogFormat::from_str("json").unwrap(), LogFormat::Json);
        assert_eq!(LogFormat::from_str("PRETTY").unwrap(), LogFormat::Pretty);
        assert!(LogFormat::from_str("invalid").is_err());
    }

    #[test]
    fn test_config_presets() {
        let dev_config = LogConfig::development();
        assert_eq!(dev_config.level, LogLevel::Debug);
        assert_eq!(dev_config.format, LogFormat::Pretty);
        assert!(dev_config.performance_tracing);

        let prod_config = LogConfig::production();
        assert_eq!(prod_config.level, LogLevel::Warn);
        assert_eq!(prod_config.format, LogFormat::Json);
        assert!(!prod_config.performance_tracing);
    }

    #[test]
    fn test_filter_creation() {
        let config = LogConfig::default();
        let filter = create_filter(&config);
        assert!(filter.is_ok());
    }

    #[test]
    fn test_performance_macro() {
        init_test_logging();
        
        let result = trace_performance!("test_operation", {
            std::thread::sleep(std::time::Duration::from_millis(10));
            42
        });
        
        assert_eq!(result, 42);
    }
}