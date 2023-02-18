#![deny(
	clippy::unwrap_used,
	clippy::absurd_extreme_comparisons,
	clippy::clone_on_copy
)]
#![warn(clippy::all, clippy::style)]
#![allow(
	clippy::collapsible_else_if,
	clippy::collapsible_if,
	clippy::comparison_chain
)]

mod cli;
mod overlay;
mod profile;

use std::process::exit;

use log::error;

fn main() {
	if let Err(e) = cli::init() {
		error!("{:#}", e);

		exit(1);
	}
}
