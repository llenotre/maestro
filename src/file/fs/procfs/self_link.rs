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

//! This module implements the `self` symlink, which points to the current
//! process's directory.

use crate::errno::EResult;
use crate::errno::Errno;
use crate::file::fs::kernfs::content::KernFSContent;
use crate::file::fs::kernfs::node::KernFSNode;
use crate::file::perm;
use crate::file::perm::Gid;
use crate::file::perm::Uid;
use crate::file::FileContent;
use crate::file::Mode;
use crate::process::Process;
use crate::time::unit::Timestamp;
use crate::util::io::IO;

/// The `self` symlink.
pub struct SelfNode {}

impl KernFSNode for SelfNode {
	fn get_hard_links_count(&self) -> u16 {
		1
	}

	fn set_hard_links_count(&mut self, _: u16) {}

	fn get_mode(&self) -> Mode {
		0o777
	}

	fn set_mode(&mut self, _: Mode) {}

	fn get_uid(&self) -> Uid {
		perm::ROOT_UID
	}

	fn set_uid(&mut self, _: Uid) {}

	fn get_gid(&self) -> Gid {
		perm::ROOT_GID
	}

	fn set_gid(&mut self, _: Gid) {}

	fn get_atime(&self) -> Timestamp {
		0
	}

	fn set_atime(&mut self, _: Timestamp) {}

	fn get_ctime(&self) -> Timestamp {
		0
	}

	fn set_ctime(&mut self, _: Timestamp) {}

	fn get_mtime(&self) -> Timestamp {
		0
	}

	fn set_mtime(&mut self, _: Timestamp) {}

	fn get_content(&mut self) -> EResult<KernFSContent<'_>> {
		let pid = Process::current_assert().lock().pid;
		let pid_string = crate::format!("{pid}")?;
		Ok(FileContent::Link(pid_string).into())
	}
}

impl IO for SelfNode {
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
