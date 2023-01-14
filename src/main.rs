#![deny(
	clippy::all,
	clippy::unwrap_used,
	clippy::absurd_extreme_comparisons,
	clippy::clone_on_copy
)]
#![allow(
	clippy::collapsible_else_if,
	clippy::collapsible_if,
	clippy::comparison_chain
)]

mod cli;
mod overlay;
mod profile;

use log::error;

fn main() {
	if let Err(e) = cli::init() {
		// when running in debug builds show a stack
		// trace for easier debugging of errors
		#[cfg(debug_assertions)]
		error!("{:?}", e);

		#[cfg(not(debug_assertions))]
		error!("{:#}", e);
	}
}
