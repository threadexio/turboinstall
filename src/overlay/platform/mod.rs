pub(self) mod prelude {
	pub use crate::cli::Options;

	pub use anyhow::Result;

	pub use std::path::Path;
}

// Unix specific implementations
#[cfg(unix)]
#[path = "unix/mod.rs"]
mod imp;

// Fallback implementations
#[cfg(all(not(unix)))]
#[path = "fallback/mod.rs"]
mod imp;

pub use imp::{copy, create_dir_all, hard_link, PlatformOptions};
