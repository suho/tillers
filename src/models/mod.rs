//! Data models for TilleRS window manager

pub mod workspace;
pub mod tiling_pattern;
pub mod window_rule;
pub mod monitor_configuration;
pub mod keyboard_mapping;
pub mod application_profile;

pub use workspace::*;
pub use tiling_pattern::*;
pub use window_rule::*;
pub use monitor_configuration::*;
pub use keyboard_mapping::*;
pub use application_profile::*;
