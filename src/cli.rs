use std::path::PathBuf;

use anyhow::{bail, Context, Result};

use clap::{CommandFactory, Parser, ValueHint};

use crate::overlay;
use crate::profile;

#[derive(Debug, Parser)]
#[clap(
	name = clap::crate_name!(),
	about = clap::crate_description!(),
	version = clap::crate_version!(),
	author = clap::crate_authors!(),
	disable_help_subcommand(true),
	subcommand_negates_reqs(true)
)]
struct Options {
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

	#[clap(flatten)]
	pub install_options: overlay::InstallOptions,

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
}

pub fn init() -> Result<()> {
	#[cfg(debug_assertions)]
	Options::command().debug_assert();

	let mut options = Options::parse();

	init_log(&options)?;

	if options.src.is_empty() {
		bail!("You must specify at least one source path.")
	}

	if options.install_options.dry_run {
		options.install_options.no_hooks = true;
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

		overlay.run_hooks(
			HookType::PreInstall,
			&options.install_options,
		)?;

		overlay
			.install(profile.as_ref(), &options.install_options)?;

		overlay.run_hooks(
			HookType::PostInstall,
			&options.install_options,
		)?;
	}

	Ok(())
}

use colored::Colorize;
fn init_log(_options: &Options) -> Result<()> {
	use log::Level;

	let fern = fern::Dispatch::new()
		.format(move |out, message, record| {
			out.finish(format_args!(
				" {} {}",
				match record.level() {
					Level::Error => "›".red(),
					Level::Warn => "›".yellow(),
					Level::Info => "›".cyan(),
					Level::Debug => "›".white(),
					Level::Trace =>
						record.level().as_str().bright_white(),
				},
				message
			))
		})
		.chain(std::io::stderr());

	fern.apply().context("Unable to initialize logger")?;

	Ok(())
}
