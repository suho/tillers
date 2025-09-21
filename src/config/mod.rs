//! Configuration management for TilleRS

pub mod parser;
pub mod persistence;
pub mod simple_persistence;
pub mod validator;

pub use parser::{ConfigFile, ConfigParseError, ConfigParser, WorkspaceConfig};
pub use persistence::{ConfigPersistence, PersistenceConfig, PersistenceError};
pub use simple_persistence::{
    SimpleConfigPersistence, SimplePersistenceConfig, SimplePersistenceError,
};
pub use validator::{ConfigValidator, ValidationResult, ValidationRule, ValidationSeverity};
