use std::path::Path;

use anyhow::{bail, Context, Result};

mod env;
mod serde;

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum Format {
	Json,
	Toml,
	Yaml,
	Env,
}

pub trait Profile {
	fn var(&self, item: &str) -> Option<&str>;
	fn list(&self) -> Vec<(String, String)>;
}

pub struct NoProfile;

impl NoProfile {
	pub fn new() -> Self {
		Self
	}
}

impl Profile for NoProfile {
	fn var(&self, _: &str) -> Option<&str> {
		None
	}

	fn list(&self) -> Vec<(String, String)> {
		Vec::new()
	}
}

pub fn load_file(
	file: &Path,
	mut fmt: Option<Format>,
) -> Result<Box<dyn Profile>> {
	let raw = std::fs::read_to_string(file).context(format!(
		"Unable to read profile '{}'",
		file.display()
	))?;

	if fmt.is_none() {
		if let Some(ext) = file.extension() {
			fmt = Some(match ext.to_string_lossy().as_ref() {
				"json" => Format::Json,
				"toml" => Format::Toml,
				"yaml" | "yml" => Format::Yaml,
				"env" => Format::Env,
				_ => bail!("Unable to determine profile format. Please use `--format`!")
			})
		}
	}

	load_str(&raw, fmt)
}

pub fn load_str(
	raw: &str,
	fmt: Option<Format>,
) -> Result<Box<dyn Profile>> {
	let profile_format;
	if let Some(fmt) = fmt {
		profile_format = fmt;
	} else {
		bail!("Unable to determine profile format. Please use `--format`!")
	}

	match profile_format {
		Format::Json => serde::SerdeProfile::from_json(raw),
		Format::Toml => serde::SerdeProfile::from_toml(raw),
		Format::Yaml => serde::SerdeProfile::from_yaml(raw),
		Format::Env => env::EnvProfile::from_str(raw),
	}
}
