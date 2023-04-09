use std::path::PathBuf;

use anyhow::{bail, Context, Result};

use clap::{CommandFactory, Parser, ValueHint};
use log::info;

use crate::overlay;
use crate::profile;

#[warn(unused)]
#[derive(Debug, Parser)]
#[clap(
	name = clap::crate_name!(),
	about = clap::crate_description!(),
	version = clap::crate_version!(),
	author = clap::crate_authors!(),
	disable_help_subcommand(true),
	subcommand_negates_reqs(true)
)]
pub struct Options {
	#[clap(
		help = "Destination directory",
		value_name("dir"),
		value_hint(ValueHint::DirPath)
	)]
	pub dst: PathBuf,

	#[clap(
		help = "Overlay source(s)",
		value_name("dir"),
		value_hint(ValueHint::DirPath)
	)]
	pub src: Vec<PathBuf>,

	#[clap(
		short = 'p',
		long = "profile",
		help = "Path to the file with the profile definition",
		default_value = ".turboinstall.json",
		value_name("/path/to/profile"),
		value_hint(ValueHint::FilePath)
	)]
	pub profile_path: PathBuf,

	#[clap(
		short = 'f',
		long = "format",
		help = "Specify which format the profile uses",
		value_name("fmt")
	)]
	pub profile_format: Option<profile::Format>,

	#[clap(
		short = 'l',
		long = "link",
		help = "Hard link files instead of copying",
		conflicts_with("update")
	)]
	pub hard_link: bool,

	#[clap(
		short = 'n',
		long = "no-clobber",
		help = "Do not overwrite existing files",
		conflicts_with("update")
	)]
	pub no_overwrite: bool,

	#[clap(
		short = 'u',
		long = "update",
		help = "Overwrite only when the source path is newer"
	)]
	pub update: bool,

	#[clap(
		short = 'q',
		long = "quiet",
		help = "Don't print anything to the console"
	)]
	pub quiet: bool,

	#[clap(
		long = "ignore",
		help = "Regex path pattern to ignore",
		value_name("pattern"),
		value_hint(ValueHint::AnyPath)
	)]
	pub ignore_patterns: Vec<String>,

	#[clap(
		long = "ignore-file",
		help = "Paths to extra ignore files",
		value_name("path,path,..."),
		value_delimiter(','),
		value_hint(ValueHint::FilePath)
	)]
	pub ignore_paths: Vec<PathBuf>,

	#[clap(long = "no-abort", help = "Don't exit on error")]
	pub no_abort: bool,

	#[clap(
		long = "dry-run",
		help = "Do not perform any filesystem operations (implies --no-hooks)"
	)]
	pub dry_run: bool,

	#[clap(long = "no-hooks", help = "Do not run any hooks")]
	pub no_hooks: bool,

	#[clap(
		long = "hooks",
		help = "Only run these types of hooks",
		value_name("type,type,..."),
		value_delimiter(',')
	)]
	pub hook_types: Vec<overlay::HookType>,

	#[clap(
		long = "porcelain",
		help = "Use machine readable output",
		conflicts_with("quiet")
	)]
	pub machine_readable: bool,

	#[clap(flatten)]
	pub platform_options: overlay::platform::PlatformOptions,
}

pub fn init() -> Result<()> {
	#[cfg(debug_assertions)]
	Options::command().debug_assert();

	let mut options = Options::parse();

	// if stdout is not a tty, then we must be being piped
	// to somewhere else, use machine readable output
	if atty::isnt(atty::Stream::Stdout) {
		options.machine_readable = true;
	}

	init_log(&options)?;

	if options.src.is_empty() {
		bail!("You must specify at least one source path.")
	}

	if options.dry_run {
		options.no_hooks = true;
	}

	// if the file does not exist default to using an empty profile
	let profile = if options.profile_path.exists() {
		profile::load_file(
			&options.profile_path,
			options.profile_format,
		)?
	} else {
		Box::new(profile::NoProfile::new())
	};

	// initialize our overlays
	let mut overlays = Vec::with_capacity(options.src.len());
	options.src.iter().try_for_each(|src| -> Result<()> {
		overlays.push(overlay::Overlay::new(src, &options.dst)?);
		Ok(())
	})?;

	for overlay in &mut overlays {
		use overlay::HookType;

		let start = std::time::Instant::now();

		overlay.run_hooks(
			HookType::PreInstall,
			&options,
			profile.as_ref(),
		)?;

		overlay.install(profile.as_ref(), &options)?;

		overlay.run_hooks(
			HookType::PostInstall,
			&options,
			profile.as_ref(),
		)?;

		info!(target: "no_fmt", "{:>12} {} overlay(s) in {:.3}s", "Finished".bold().bright_green(), options.profile_path.to_string_lossy().dimmed(), start.elapsed().as_secs_f64());
	}

	Ok(())
}

use colored::Colorize;
fn init_log(options: &Options) -> Result<()> {
	use log::Level;

	let mut fern =
		fern::Dispatch::new().format(move |out, message, record| {
			if record.target() == "no_fmt" {
				out.finish(format_args!("{}", message))
			} else {
				out.finish(format_args!(
					"{:>12} {}",
					match record.level() {
						Level::Error => "Error".bold().bright_red(),
						Level::Warn =>
							"Warning".bold().bright_yellow(),
						Level::Info => "Info".bold().bright_green(),
						Level::Debug =>
							"Verbose".bold().bright_white(),
						Level::Trace => "Trace".bold().bright_white(),
					},
					message
				))
			}
		});

	if !options.quiet {
		fern = fern.chain(std::io::stderr());
	}

	fern.apply().context("Unable to initialize logger")?;

	Ok(())
}
