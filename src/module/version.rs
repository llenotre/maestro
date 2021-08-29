//! The module implements a Version structure.
//! A version is divided into the following component:
//! - Major: Version including breaking changes
//! - Minor: Version including new features
//! - Patch: Version including bug fixes and optimizations

use core::cmp::Ordering;

/// Structure representing a version.
#[derive(Clone, Debug, Eq)]
pub struct Version {
	/// The major version
	pub major: u16,
	/// The minor version
	pub minor: u16,
	/// The patch version
	pub patch: u16,
}

impl Version {
	/// Compares current version with the given one.
	fn cmp(&self, other: &Self) -> Ordering {
		let mut ord = self.major.cmp(&other.major);
		if ord != Ordering::Equal {
			return ord;
		}

		ord = self.minor.cmp(&other.minor);
		if ord != Ordering::Equal {
			return ord;
		}

		self.patch.cmp(&other.patch)
	}

	// TODO to_string
}

impl Ord for Version {
	fn cmp(&self, other: &Self) -> Ordering {
		self.cmp(other)
	}
}

impl PartialOrd for Version {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl PartialEq for Version {
	fn eq(&self, other: &Self) -> bool {
		self.major == other.major && self.minor == other.minor && self.patch == other.patch
	}
}

// TODO Implement fmt::Display