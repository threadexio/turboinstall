use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use colored::Colorize;
use log::{error, info, warn};

use crate::cli::Options;
use crate::profile::Profile;

mod ignore;
pub mod platform;

const DEFAULT_IGNORE: &[&str] = &["^/.turboinstall"];

#[derive(Debug, Clone, PartialEq, Eq, clap::ValueEnum)]
pub enum HookType {
	PreInstall,
	PostInstall,
}

impl HookType {
	pub(self) fn hook_dir_name(&self) -> &str {
		match self {
			HookType::PreInstall => "pre-install",
			HookType::PostInstall => "post-install",
		}
	}
}

#[derive(Debug)]
pub struct Overlay {
	src: PathBuf,
	dst: PathBuf,
}

impl Overlay {
	pub fn new(
		src: impl AsRef<Path>,
		dst: impl AsRef<Path>,
	) -> Result<Self> {
		let src = src.as_ref();
		let dst = dst.as_ref();

		let src = src.canonicalize().with_context(move || {
			format!("'{}' does not exist", src.display())
		})?;

		if !src.is_dir() {
			bail!("'{}' is not a directory", src.display())
		}

		let dst = dst.canonicalize().with_context(move || {
			format!("'{}' does not exist", dst.display())
		})?;

		if !dst.is_dir() {
			bail!("'{}' is not a directory", dst.display())
		}

		if src.ancestors().any(|x| x == dst) {
			bail!(
				"source '{}' cannot be in destination '{}'",
				src.display(),
				dst.display()
			)
		}

		if dst.ancestors().any(|x| x == src) {
			bail!(
				"destination '{}' cannot be in source '{}'",
				dst.display(),
				src.display()
			)
		}

		Ok(Self { src, dst })
	}

	pub fn install(
		&mut self,
		profile: &dyn Profile,
		options: &Options,
	) -> Result<()> {
		let mut ignore = ignore::Ignore::empty();

		// default ignores
		for pattern in DEFAULT_IGNORE {
			ignore.add_pattern(pattern).with_context(|| format!("Default pattern '{}' failed to compile. This is a bug!", pattern))?;
		}

		// load ignore file if it exists
		{
			// if the ignore path is absolute it will overwrite the self.src prefix
			// and thus correctly use the absolute path
			let ignore_path = self.src.join(&options.ignore_path);
			if ignore_path.exists() {
				ignore.add_from_file(ignore_path)?;
			}
		}

		let relative_paths = walkdir::WalkDir::new(&self.src)
			// dont return self.src again
			.min_depth(1)
			.contents_first(false)
			.follow_links(false)
			.sort_by_file_name()
			.into_iter()
			// filter out all the problem entries
			.filter_map(|x| x.ok())
			.filter_map(|x| {
				// convert to path relative to &self.src
				x.path()
					.strip_prefix(&self.src)
					.map(|x| x.to_path_buf())
					.ok()
			})
			.filter(|x| {
				// we add a / in front of the relative path
				// so we can use the leading / to match files
				// in the root of the overlay
				let absolute_path = Path::new("/").join(x);
				!ignore.matches(absolute_path.to_string_lossy())
			});

		for src_rel_path in relative_paths {
			let r = self.install_path(src_rel_path, profile, options);

			if options.no_abort {
				if let Err(e) = r {
					error!("{} {:#}", "[Silent]".dimmed().white(), e);
				}
			} else {
				r?
			}
		}

		Ok(())
	}

	fn install_path(
		&self,
		src_rel_path: PathBuf,
		profile: &dyn Profile,
		options: &Options,
	) -> Result<()> {
		let dst_rel_path = expand_path(&src_rel_path, profile)
			.with_context(|| {
				format!(
					"failed to expand path '{}'",
					src_rel_path.display()
				)
			})?;

		let src = self.src.join(&src_rel_path);
		let dst = self.dst.join(&dst_rel_path);

		let src_metadata = src.metadata().with_context(|| {
			format!("failed to get metadata for '{}'", src.display())
		})?;

		if dst.exists() {
			let dst_metadata = dst.metadata().with_context(|| {
				format!(
					"failed to get metadata for '{}'",
					dst.display()
				)
			})?;

			if options.update {
				let now = std::time::SystemTime::now();

				let src_mtime =
					src_metadata.modified().unwrap_or(now);
				let dst_mtime =
					dst_metadata.modified().unwrap_or(now);

				if src_mtime < dst_mtime {
					warn!(
						"destination '{}' is newer than source '{}'",
						dst.display(),
						src.display(),
					);
					return Ok(());
				} else if src_mtime == dst_mtime {
					// dont do unnecessary operations
					return Ok(());
				}
			}

			if options.no_overwrite {
				warn!(
					"not overwriting existing path '{}'",
					dst.display()
				);
				return Ok(());
			}
		}

		if !options.dry_run {
			if src.is_dir() {
				platform::create_dir_all(&dst, options)
					.with_context(|| {
						format!(
							"failed to create directory '{}'",
							dst.display()
						)
					})?;
			} else {
				if options.hard_link {
					platform::hard_link(&src, &dst, options)
						.with_context(|| {
							format!(
								"failed to hard link '{}' to '{}'",
								src.display(),
								dst.display()
							)
						})?
				} else {
					platform::copy(&src, &dst, options)
						.with_context(|| {
							format!(
								"failed to install '{}' to '{}'",
								src.display(),
								dst.display()
							)
						})?;
				}
			}
		}

		if options.machine_readable {
			println!("{} {}", src.display(), dst.display());
		} else {
			info!(target: "no_fmt", "{:>12} {} {} {}", "Installing".bold().bright_green(), src.display(), "to".bold().bright_cyan(), dst.display());
		}

		Ok(())
	}

	pub fn run_hooks(
		&mut self,
		hook_type: HookType,
		options: &Options,
	) -> Result<()> {
		if options.no_hooks {
			return Ok(());
		}

		// if it is empty, then run any hook type
		if !options.hook_types.is_empty()
			&& !options.hook_types.contains(&hook_type)
		{
			return Ok(());
		}

		let hook_dir = self
			.src
			.join(".turboinstall")
			.join(hook_type.hook_dir_name());

		if !hook_dir.exists() {
			return Ok(());
		}

		if !hook_dir.is_dir() {
			bail!(
				"hook directory '{}' is not a directory",
				hook_dir.display()
			)
		}

		// iteratively run hooks in alphanumerical order
		walkdir::WalkDir::new(hook_dir)
			.max_depth(1)
			.follow_links(true)
			.contents_first(true)
			.sort_by_file_name()
			.into_iter()
			.filter_map(|x| x.ok())
			.map(|x| x.into_path())
			.filter(|x| x.is_file())
			.try_for_each(move |hook_path| {
				use std::process::Command;

				info!(target: "no_fmt", "{:>12} {}", "Running".bold().bright_white(), hook_path.display());

				let status = match Command::new(&hook_path)
					.arg(&self.src)
					.arg(&self.dst)
					.status()
				{
					Ok(v) => v,
					Err(_) => {
						warn!(
							"could not run hook '{}'",
							hook_path.display()
						);
						return Ok(());
					},
				};

				if !status.success() {
					if let Some(code) = status.code() {
						bail!(
							"hook '{}' exited with code: {}",
							hook_path.display(),
							code
						)
					} else {
						bail!("hook '{}' failed", hook_path.display())
					}
				}

				Ok(())
			})?;

		Ok(())
	}
}

fn expand_vars(s: &str, profile: &dyn Profile) -> Result<String> {
	let mut ret = s.to_string();

	loop {
		let start = ret.find('{');
		let end = ret.find('}');

		if !(start.is_some() && end.is_some()) {
			break;
		}

		if end <= start {
			break;
		}

		let start = start.expect("the None should have been handled by the above if statements");
		let end = end.expect("the None should have been handled by the above if statements");

		let var_name = &ret[start.saturating_add(1)..end];

		if let Some(value) = profile.var(var_name) {
			if value.is_empty() {
				bail!("Found empty variable.")
			}

			ret = format!(
				"{}{}{}",
				&ret[..start],
				value,
				&ret[end.saturating_add(1)..]
			);
		} else {
			bail!("Variable '{}' not found in profile.", var_name)
		}
	}

	Ok(ret)
}

fn expand_path(
	p: impl AsRef<Path>,
	profile: &dyn Profile,
) -> Result<PathBuf> {
	let mut path = PathBuf::new();

	for component in p
		.as_ref()
		.components()
		.map(|x| x.as_os_str().to_string_lossy())
	{
		let expanded = expand_vars(&component, profile)?;

		let expanded =
			expanded.strip_prefix('/').unwrap_or(&expanded);

		path.push(expanded);
	}

	Ok(path)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn expand_vars_tests() {
		use std::collections::HashMap;

		impl Profile for HashMap<String, String> {
			fn var(&self, s: &str) -> Option<&str> {
				self.get(s).map(|x| x.as_str())
			}
		}

		let mut dummy_profile: HashMap<String, String> =
			HashMap::new();
		dummy_profile
			.insert("var1".to_string(), "variable 1".to_string());
		dummy_profile
			.insert("VAR2".to_string(), "VARIABLE 2".to_string());
		dummy_profile
			.insert("space var".to_string(), " spaced ".to_string());

		assert_eq!(
			expand_vars("..{var1}..", &dummy_profile).unwrap(),
			"..variable 1.."
		);

		assert_eq!(
			expand_vars("{var1}..", &dummy_profile).unwrap(),
			"variable 1.."
		);

		assert_eq!(
			expand_vars("..{var1}", &dummy_profile).unwrap(),
			"..variable 1"
		);

		assert_eq!(
			expand_vars("{VAR2}", &dummy_profile).unwrap(),
			"VARIABLE 2"
		);

		assert_eq!(
			expand_vars("..{space var}..", &dummy_profile).unwrap(),
			".. spaced .."
		);

		assert_eq!(
			expand_vars("{var1} {VAR2}", &dummy_profile).unwrap(),
			"variable 1 VARIABLE 2"
		);

		assert_eq!(
			expand_vars("}var1{", &dummy_profile).unwrap(),
			"}var1{"
		);

		assert!(expand_vars("{}{var1}{}", &dummy_profile).is_err());
		assert!(expand_vars("{}}var1{{}", &dummy_profile).is_err());
	}
}
