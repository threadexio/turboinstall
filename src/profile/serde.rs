use std::collections::HashMap;

use anyhow::{Context, Result};

use super::Profile;

#[derive(Debug, serde::Deserialize)]
pub struct SerdeProfile {
	#[serde(flatten)]
	inner: HashMap<String, String>,
}

impl SerdeProfile {
	pub fn from_json(s: &str) -> Result<Box<dyn Profile>> {
		Ok(Box::new(
			serde_json::from_str::<Self>(s)
				.context("Unable to parse json")?,
		))
	}

	pub fn from_toml(s: &str) -> Result<Box<dyn Profile>> {
		Ok(Box::new(
			toml::from_str::<Self>(s)
				.context("Unable to parse toml")?,
		))
	}

	pub fn from_yaml(s: &str) -> Result<Box<dyn Profile>> {
		Ok(Box::new(
			serde_yaml::from_str::<Self>(s)
				.context("Unable to parse yaml")?,
		))
	}
}

impl Profile for SerdeProfile {
	fn var(&self, item: &str) -> Option<&str> {
		self.inner.get(item).map(|x| x.as_str())
	}
}
