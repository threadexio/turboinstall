use super::prelude::*;

#[derive(Debug, clap::Args)]
pub struct PlatformOptions {}

pub fn create_dir_all(dst: &Path, _: &Options) -> Result<()> {
	todo!()
}

pub fn hard_link(src: &Path, dst: &Path, _: &Options) -> Result<()> {
	todo!()
}

pub fn copy(src: &Path, dst: &Path, _: &Options) -> Result<()> {
	todo!()
}
