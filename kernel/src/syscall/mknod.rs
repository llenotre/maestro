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

//! The `mknod` system call allows to create a new node on a filesystem.

use crate::{
	device::id,
	file,
	file::{
		path::{Path, PathBuf},
		vfs,
		vfs::ResolutionSettings,
		FileContent, FileType,
	},
	process::{mem_space::ptr::SyscallString, Process},
};
use macros::syscall;
use utils::{errno, errno::Errno};

// TODO Check args type
#[syscall]
pub fn mknod(pathname: SyscallString, mode: file::Mode, dev: u64) -> Result<i32, Errno> {
	let (path, umask, rs) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let path = pathname.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;
		let path = PathBuf::try_from(path)?;

		let umask = proc.umask;

		let rs = ResolutionSettings::for_process(&proc, true);
		(path, umask, rs)
	};

	// Path of the parent directory
	let parent_path = path.parent().unwrap_or(Path::root());
	// File name
	let Some(name) = path.file_name() else {
		return Err(errno!(EEXIST));
	};

	let mode = mode & !umask;
	let file_type = FileType::from_mode(mode).ok_or(errno!(EPERM))?;

	// Get the major and minor IDs
	let major = id::major(dev);
	let minor = id::minor(dev);

	// The file's content
	let file_content = match file_type {
		FileType::Regular => FileContent::Regular,
		FileType::Fifo => FileContent::Fifo,
		FileType::Socket => FileContent::Socket,
		FileType::BlockDevice => FileContent::BlockDevice {
			major,
			minor,
		},
		FileType::CharDevice => FileContent::CharDevice {
			major,
			minor,
		},
		_ => return Err(errno!(EPERM)),
	};

	// Create the node
	let parent_mutex = vfs::get_file_from_path(parent_path, &rs)?;
	let mut parent = parent_mutex.lock();
	vfs::create_file(
		&mut parent,
		name.try_into()?,
		&rs.access_profile,
		mode,
		file_content,
	)?;

	Ok(0)
}