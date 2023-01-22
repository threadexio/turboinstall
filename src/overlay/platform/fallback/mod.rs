use super::prelude::*;

use std::fs;

#[derive(Debug, clap::Args)]
pub struct PlatformOptions {}

pub fn create_dir_all(
	src: &Path,
	dst: &Path,
	_: &Options,
) -> Result<()> {
	let src_metadata = src.metadata()?;
	fs::create_dir_all(dst)?;

	fs::set_permissions(dst, src_metadata.permissions())
		.context("failed to preserve permissions")?;

	Ok(())
}

// the `remove_file_if_exists` handle the fact that `fs::hard_link` fails
// if dst already exists
// and that trying to copy after linking will not remove
// the link

pub fn hard_link(src: &Path, dst: &Path, _: &Options) -> Result<()> {
	let _ = remove_file_if_exists(dst);
	fs::hard_link(&src, &dst)?;
	Ok(())
}

pub fn copy(src: &Path, dst: &Path, _: &Options) -> Result<()> {
	let _ = remove_file_if_exists(dst);
	fs::copy(&src, &dst)?;
	Ok(())
}

fn remove_file_if_exists(path: &Path) -> Result<()> {
	if path.exists() {
		fs::remove_file(&path)?;
	}

	Ok(())
}
