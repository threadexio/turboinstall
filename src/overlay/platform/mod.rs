// Unix specific implementations
#[cfg(all(not(feature = "no-platform-specific"), unix))]
#[path = "unix/mod.rs"]
mod imp;

// Fallback implementations
#[cfg(any(feature = "no-platform-specific", not(unix)))]
#[path = "fallback/mod.rs"]
mod imp;

pub use imp::{copy, create_dir_all, hard_link, PlatformOptions};
