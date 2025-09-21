//! Configuration management for TilleRS

pub mod parser;
pub mod validator;
pub mod persistence;
pub mod simple_persistence;

pub use parser::{ConfigParser, ConfigParseError, WorkspaceConfig, ConfigFile};
pub use validator::{ConfigValidator, ValidationResult, ValidationRule, ValidationSeverity};
pub use persistence::{ConfigPersistence, PersistenceConfig, PersistenceError};
pub use simple_persistence::{SimpleConfigPersistence, SimplePersistenceConfig, SimplePersistenceError};
