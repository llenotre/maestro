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

use crate::device::id;
use crate::errno;
use crate::errno::Errno;
use crate::file;
use crate::file::path::Path;
use crate::file::vfs;
use crate::file::FileContent;
use crate::file::FileType;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use macros::syscall;

// TODO Check args type
#[syscall]
pub fn mknod(pathname: SyscallString, mode: file::Mode, dev: u64) -> Result<i32, Errno> {
	let (path, umask, ap) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let path = Path::from_str(pathname.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?, true)?;
		let path = super::util::get_absolute_path(&proc, path)?;

		let umask = proc.umask;

		(path, umask, proc.access_profile)
	};

	// Path of the parent directory
	let mut parent_path = path;
	// File name
	let Some(name) = parent_path.pop() else {
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
	let parent_mutex = vfs::get_file_from_path(&parent_path, &ap, true)?;
	let mut parent = parent_mutex.lock();
	vfs::create_file(&mut parent, name, &ap, mode, file_content)?;

	Ok(0)
}
