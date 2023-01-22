use super::prelude::*;

use std::fs;
use std::os::unix::prelude::*;

use anyhow::{bail, Context};

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum Preserve {
	Ownership,
	Timestamps,
	// TODO: Maybe also xattrs
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum Reflink {
	Never,
	Always,
	Auto,
}

#[derive(Debug, clap::Args)]
pub struct PlatformOptions {
	#[clap(
		long = "preserve",
		help = "Preserve the specified attributes",
		value_name("attr,attr,..."),
		value_delimiter(','),
		conflicts_with("hard_link")
	)]
	preserve: Vec<Preserve>,

	#[clap(
		long = "reflink",
		help = "Create clone/CoW copies",
		default_value = "auto",
		value_name("when"),
		conflicts_with("hard_link")
	)]
	reflink: Reflink,
}

pub fn create_dir_all(
	src_path: &Path,
	dst_path: &Path,
	options: &Options,
) -> Result<()> {
	let src_metadata = src_path.metadata()?;
	fs::create_dir_all(dst_path)?;

	fs::set_permissions(dst_path, src_metadata.permissions())
		.context("failed to preserve permissions")?;

	for p in &options.platform_options.preserve {
		match p {
			Preserve::Ownership => {
				use nix::unistd::{fchownat, FchownatFlags, Gid, Uid};

				let uid = src_metadata.uid();
				let gid = src_metadata.gid();

				fchownat(
					None,
					dst_path,
					Some(Uid::from_raw(uid)),
					Some(Gid::from_raw(gid)),
					FchownatFlags::NoFollowSymlink,
				)
				.context("failed to preserve ownership")?;
			},
			Preserve::Timestamps => {
				use nix::sys::{stat::utimensat, stat::UtimensatFlags, time::TimeSpec};

				let atime = TimeSpec::new(
					src_metadata.atime(),
					src_metadata.atime_nsec(),
				);

				let mtime = TimeSpec::new(
					src_metadata.mtime(),
					src_metadata.mtime_nsec(),
				);

				utimensat(None, dst_path, &atime, &mtime, UtimensatFlags::NoFollowSymlink)
					.context("failed to preserve timestamps")?;
			},
		}
	}

	Ok(())
}

pub fn hard_link(
	src_path: &Path,
	dst_path: &Path,
	_: &Options,
) -> Result<()> {
	fs::hard_link(src_path, dst_path)?;
	Ok(())
}

pub fn copy(
	src_path: &Path,
	dst_path: &Path,
	options: &Options,
) -> Result<()> {
	let src_metadata = src_path.metadata()?;

	let mut src = fs::OpenOptions::new()
		.read(true)
		.open(src_path)
		.context("failed to open source")?;
	let mut dst = fs::OpenOptions::new()
		.write(true)
		.create(true)
		.truncate(true)
		.custom_flags(nix::libc::O_CLOEXEC)
		.open(dst_path)
		.context("failed to open destination")?;

	// decide if we are going to use reflinks
	{
		let reflink = {
			let dst_fd = dst.as_raw_fd();
			let src_fd = src.as_raw_fd();
			move || reflink(dst_fd, src_fd)
		};

		let mut copy_file = || -> Result<()> {
			// tell the kernel we are going to need dst
			// so it can load it in memory
			nix::fcntl::posix_fadvise(
				dst.as_raw_fd(),
				0,
				i64::MAX,
				nix::fcntl::PosixFadviseAdvice::POSIX_FADV_SEQUENTIAL,
			)
			.context("failed to preload destination")?;

			// std::fs::copy also copies the permissions, so we have to guarantee the same behavior
			fs::set_permissions(dst_path, src_metadata.permissions())
				.context("failed to preserve permissions")?;

			std::io::copy(&mut src, &mut dst)
				.context("failed to copy file data")?;

			Ok(())
		};

		match options.platform_options.reflink {
			Reflink::Always => {
				if !reflink() {
					bail!("failed to create reflink");
				}
			},
			Reflink::Never => {
				copy_file()?;
			},
			Reflink::Auto => {
				if !reflink() {
					copy_file()?;
				}
			},
		}
	}

	// preserve attributes
	for p in &options.platform_options.preserve {
		match p {
			Preserve::Ownership => {
				use nix::unistd::{fchown, Gid, Uid};

				let uid = src_metadata.uid();
				let gid = src_metadata.gid();

				fchown(
					dst.as_raw_fd(),
					Some(Uid::from_raw(uid)),
					Some(Gid::from_raw(gid)),
				)
				.context("failed to preserve ownership")?;
			},
			Preserve::Timestamps => {
				use nix::sys::{stat::futimens, time::TimeSpec};

				let atime = TimeSpec::new(
					src_metadata.atime(),
					src_metadata.atime_nsec(),
				);

				let mtime = TimeSpec::new(
					src_metadata.mtime(),
					src_metadata.mtime_nsec(),
				);

				futimens(dst.as_raw_fd(), &atime, &mtime)
					.context("failed to preserve timestamps")?;
			},
		}
	}

	Ok(())
}

/// Reflink `src` to `dst`
///
/// Equivalent to: `ioctl(dst, FICLONE, src)`
///
/// Returns:
/// Whether the `ioctl()` call succeeded
fn reflink(dst: RawFd, src: RawFd) -> bool {
	unsafe { nix::libc::ioctl(dst, nix::libc::FICLONE, src) == 0 }
}
