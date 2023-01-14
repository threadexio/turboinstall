pub(self) mod prelude {
	pub use crate::cli::Options;

	pub use anyhow::Result;

	pub use std::path::Path;
}

// TODO: Finish NT specific implementations
// Windows specific implementations
//#[cfg(windows)]
//#[path = "nt/mod.rs"]
//mod imp;

// TODO: Finish Unix specific implementations
// Unix specific implementations
//#[cfg(unix)]
//#[path = "unix/mod.rs"]
//mod imp;

// Fallback implementations
//#[cfg(all(not(windows), not(unix)))]
#[path = "fallback/mod.rs"]
mod imp;

pub use imp::*;
