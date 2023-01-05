use std::collections::HashMap;

use anyhow::{bail, Result};

use super::Profile;

#[derive(Debug)]
pub struct EnvProfile {
	inner: HashMap<String, String>,
}

impl EnvProfile {
	pub fn from_str(s: &str) -> Result<Box<dyn Profile>> {
		let mut profile = Self { inner: HashMap::new() };

		for (i, line) in s.lines().enumerate() {
			let i = i + 1;
			let mut line = line.trim();

			if line.starts_with('#') {
				continue;
			}

			// remove quotes if any
			line = line.trim_matches('\'');
			line = line.trim_matches('\"');

			if let Some((k, v)) = line.split_once('=') {
				if k.is_empty() {
					bail!("Missing variable name at line {i}.")
				}

				profile.inner.insert(k.to_string(), v.to_string());
			} else {
				bail!("Missing assignment operator at line {i}.")
			}
		}

		Ok(Box::new(profile))
	}
}

impl Profile for EnvProfile {
	fn var(&self, item: &str) -> Option<&str> {
		self.inner.get(item).map(|x| x.as_str())
	}
}
