//! macOS integration layer for TilleRS
//!
//! These modules provide safe, testable abstractions over the macOS
//! Accessibility, Core Graphics, and Objective-C runtime APIs. The concrete
//! implementations can interact with the platform while unit tests can rely on
//! in-memory stubs or mocks.

pub mod accessibility;
pub mod core_graphics;
pub mod objc_bridge;
pub mod permissions;

pub use accessibility::*;
pub use core_graphics::*;
pub use objc_bridge::*;
pub use permissions::*;
