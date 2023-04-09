use std::ops::Deref;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use colored::Colorize;
use log::{error, info, warn};

use crate::cli::Options;
use crate::profile::Profile;

mod ignore;
pub mod platform;

static DEFAULT_IGNORE_FILES: &[&str] = &[".turboinstall/ignore"];

static DEFAULT_IGNORE_PATTERNS: &[&str] = &["^/.turboinstall"];

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
	src_root: PathBuf,
	dst_root: PathBuf,
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

		Ok(Self { src_root: src, dst_root: dst })
	}

	pub fn install(
		&mut self,
		profile: &dyn Profile,
		options: &Options,
	) -> Result<()> {
		let mut ignore = ignore::Ignore::empty();

		// default ignores
		for pattern in DEFAULT_IGNORE_PATTERNS
			.iter()
			.map(|x| x.deref())
			.chain(options.ignore_patterns.iter().map(|x| x.as_str()))
		{
			ignore.add_pattern(pattern).with_context(|| {
				format!("Failed to compile pattern `{}`", pattern)
			})?;
		}

		// load ignore files if they exists
		{
			// if the ignore path is absolute it will overwrite the self.src prefix
			// and thus correctly use the absolute path
			for ignore_path in
				DEFAULT_IGNORE_FILES.iter().map(Path::new).chain(
					options.ignore_paths.iter().map(|x| x.as_path()),
				) {
				let ignore_path = self.src_root.join(ignore_path);

				if ignore_path.exists() {
					ignore.add_from_file(&ignore_path).with_context(
						|| {
							format!(
								"Failed to read ignore file `{}`",
								ignore_path.display()
							)
						},
					)?;
				}
			}
		}

		walkdir::WalkDir::new(&self.src_root)
			// dont return self.src again
			.min_depth(1)
			.contents_first(false)
			.follow_links(false)
			.sort_by_file_name()
			.into_iter()
			// filter out all the problem entries
			.filter_map(|x| x.ok())
			.filter_map(|x| {
				// convert path to relative to &self.src_root
				x.into_path()
					.strip_prefix(&self.src_root)
					.map(|x| x.to_path_buf())
					.ok()
			})
			.filter(|x| {
				// we add a / in front of the relative path
				// so we can use the leading / to match files
				// in the root of the overlay
				let absolute_path = Path::new("/").join(x);
				!ignore.matches(absolute_path.to_string_lossy())
			}).try_for_each(|src_rel_path| -> Result<()> {
				let src = self.get_src_path(&src_rel_path).with_context(|| format!("Failed to resolve source path `{}`", src_rel_path.display()))?;
				let dst = self.get_dst_path(&src_rel_path, profile).with_context(|| format!("Failed to resolve path `{}`", src_rel_path.display()))?;

				let r = self.install_path(&src, &dst, options).with_context(|| format!("{}", src.display()));

				if options.no_abort {
					if let Err(e) = r {
						error!("{} {:#}", "[Silent]".dimmed().white(), e);
					}
				} else {
					r?
				}

				Ok(())
			})
	}

	fn get_src_path(&self, src_rel_path: &Path) -> Result<PathBuf> {
		let src = self.src_root.join(src_rel_path).canonicalize()?;
		Ok(src)
	}

	fn get_dst_path(
		&self,
		src_rel_path: &Path,
		profile: &dyn Profile,
	) -> Result<PathBuf> {
		let dst_rel_path = expand_path(src_rel_path, profile)?;
		let dst = self.dst_root.join(dst_rel_path);
		Ok(dst)
	}

	fn install_path(
		&self,
		src: &Path,
		dst: &Path,
		options: &Options,
	) -> Result<()> {
		let src_metadata =
			src.metadata().context("Failed to get metadata")?;

		if dst.exists() {
			if options.no_overwrite {
				warn!(
					"Not overwriting existing destination `{}`",
					dst.display()
				);
				return Ok(());
			}

			let dst_metadata = dst.metadata().with_context(|| {
				format!(
					"Failed to get destination `{}` metadata",
					dst.display()
				)
			})?;

			if options.update {
				let now = std::time::SystemTime::now();

				let src_mtime =
					src_metadata.modified().unwrap_or(now);
				let dst_mtime =
					dst_metadata.modified().unwrap_or(now);

				if dst_mtime > src_mtime {
					warn!("Destination `{}` is newer", dst.display(),);
					return Ok(());
				} else if dst_mtime == src_mtime {
					return Ok(());
				}
			}
		}

		if !options.dry_run {
			if src.is_dir() {
				platform::create_dir_all(src, dst, options)
					.with_context(|| {
						format!(
							"Failed to create directory `{}`",
							dst.display()
						)
					})?;
			} else {
				if options.hard_link {
					platform::hard_link(src, dst, options)
						.with_context(|| {
							format!(
								"Failed to hard link to `{}`",
								dst.display()
							)
						})?
				} else {
					platform::copy(src, dst, options).with_context(
						|| {
							format!(
								"Failed to install to `{}`",
								dst.display()
							)
						},
					)?;
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
		profile: &dyn Profile,
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
			.src_root
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


				let mut command = Command::new(&hook_path);
				command.arg(&self.src_root).arg(&self.dst_root);

				for (k, v) in profile.list() {
					command.env(k, v);
				}

				let status = match command.status()
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

			fn list(&self) -> Vec<(String, String)> {
				self.iter()
					.map(|(k, v)| (k.clone(), v.clone()))
					.collect()
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
