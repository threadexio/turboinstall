use std::collections::LinkedList;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use regex::{Regex, RegexBuilder};

#[derive(Debug)]
pub struct Ignore {
	patterns: LinkedList<Regex>,
}

impl Ignore {
	pub fn empty() -> Self {
		Self { patterns: LinkedList::new() }
	}

	pub fn add_from_str(
		&mut self,
		s: impl AsRef<str>,
	) -> Result<usize> {
		let mut patterns_added: usize = 0;

		for line in s.as_ref().lines().map(|x| x.trim()) {
			// comments and empty lines
			if line.starts_with('#') || line.is_empty() {
				continue;
			}

			self.add_pattern(line)?;
			patterns_added = patterns_added.saturating_add(1);
		}

		Ok(patterns_added)
	}

	pub fn add_from_file(
		&mut self,
		file: impl AsRef<Path>,
	) -> Result<usize> {
		let contents = fs::read_to_string(file.as_ref())
			.with_context(|| {
				format!(
					"failed to read ignore file '{}'",
					file.as_ref().display()
				)
			})?;

		self.add_from_str(&contents)
	}

	pub fn add_pattern(
		&mut self,
		pattern: impl AsRef<str>,
	) -> Result<()> {
		self.patterns
			.push_back(Self::compile_pattern(pattern.as_ref())?);
		Ok(())
	}

	pub fn matches(&self, path: impl AsRef<str>) -> bool {
		let path = path.as_ref();

		self.patterns.iter().any(|pattern| pattern.is_match(path))
	}

	fn compile_pattern(pattern: &str) -> Result<Regex> {
		RegexBuilder::new(pattern)
			.case_insensitive(false)
			.build()
			.with_context(|| {
				format!("failed to compile regex '{}'", pattern)
			})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_patterns() {
		let patterns = r"
# comment

^/file
^/dir/.*\.ignore
^/dir/[0-9]-[a-z]\.tar
^/dir/[0-9]-[a-z]\.tar\..
^/dir/dir/
		";

		let mut ignore = Ignore::empty();
		ignore.add_from_str(patterns).unwrap();

		assert!(ignore.matches("/file"));
		assert!(!ignore.matches("/dir/file"));
		assert!(!ignore.matches("file"));

		assert!(!ignore.matches("/dir"));
		assert!(ignore.matches("/dir/file1.ignore"));
		assert!(ignore.matches("/dir/file2.ignore"));
		assert!(ignore.matches("/dir/another_file.ignore"));
		assert!(!ignore.matches("/dir/test.txt"));

		assert!(ignore.matches("/dir/0-a.tar"));
		assert!(!ignore.matches("/dir/0-A.tar"));
		assert!(!ignore.matches("/dir/test.tar"));
		assert!(ignore.matches("/dir/0-a.tar.t"));
		assert!(ignore.matches("/dir/0-a.tar.e"));

		assert!(!ignore.matches("/dir1"));
		assert!(!ignore.matches("/dir/dir1"));
		assert!(!ignore.matches("/dir/dir"));
		assert!(ignore.matches("/dir/dir/"));
		assert!(ignore.matches("/dir/dir/test_file"));

		assert!(!ignore.matches("some_random_file"));
	}
}
