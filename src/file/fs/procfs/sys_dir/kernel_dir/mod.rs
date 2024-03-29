/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! TODO doc

mod osrelease;

use super::kernfs::KernFS;
use crate::errno::EResult;
use crate::errno::Errno;
use crate::file::fs::kernfs::content::KernFSContent;
use crate::file::fs::kernfs::node::KernFSNode;
use crate::file::perm::Gid;
use crate::file::perm::Uid;
use crate::file::DirEntry;
use crate::file::FileContent;
use crate::file::FileType;
use crate::file::Mode;
use crate::util::boxed::Box;
use crate::util::container::hashmap::HashMap;
use crate::util::io::IO;
use osrelease::OsRelease;

// TODO Handle dropping
/// Structure representing the `kernel` directory.
pub struct KernelDir {
	/// The content of the directory. This will always be a Directory variant.
	content: FileContent,
}

impl KernelDir {
	/// Creates a new instance.
	///
	/// The function adds every nodes to the given kernfs `fs`.
	pub fn new(fs: &mut KernFS) -> Result<Self, Errno> {
		let mut entries = HashMap::new();

		// TODO Add every nodes
		// TODO On fail, remove previously inserted nodes

		// Creating /proc/sys/kernel
		let node = OsRelease {};
		let inode = fs.add_node(Box::new(node)?)?;
		entries.insert(
			b"osrelease".try_into()?,
			DirEntry {
				inode,
				entry_type: FileType::Regular,
			},
		)?;

		Ok(Self {
			content: FileContent::Directory(entries),
		})
	}
}

impl KernFSNode for KernelDir {
	fn get_mode(&self) -> Mode {
		0o555
	}

	fn get_uid(&self) -> Uid {
		0
	}

	fn get_gid(&self) -> Gid {
		0
	}

	fn get_content(&mut self) -> EResult<KernFSContent<'_>> {
		Ok(KernFSContent::Owned(&mut self.content))
	}
}

impl IO for KernelDir {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&mut self, _offset: u64, _buff: &mut [u8]) -> Result<(u64, bool), Errno> {
		Err(errno!(EINVAL))
	}

	fn write(&mut self, _offset: u64, _buff: &[u8]) -> Result<u64, Errno> {
		Err(errno!(EINVAL))
	}

	fn poll(&mut self, _mask: u32) -> Result<u32, Errno> {
		Err(errno!(EINVAL))
	}
}
