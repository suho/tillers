//! Data models for TilleRS window manager

pub mod application_profile;
pub mod keyboard_mapping;
pub mod monitor_configuration;
pub mod tiling_pattern;
pub mod window_rule;
pub mod workspace;

pub use application_profile::*;
pub use keyboard_mapping::*;
pub use monitor_configuration::*;
pub use tiling_pattern::*;
pub use window_rule::*;
pub use workspace::*;
