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

//! The `chown` system call changes the owner of a file.

use crate::errno::EResult;
use crate::errno::Errno;
use crate::file::path::Path;
use crate::file::vfs;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;

/// Performs the `chown` syscall.
pub fn do_chown(
	pathname: SyscallString,
	owner: c_int,
	group: c_int,
	follow_links: bool,
) -> EResult<i32> {
	if owner < -1 || group < -1 {
		return Err(errno!(EINVAL));
	}

	let (path, ap) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space = mem_space.lock();

		let path = pathname.get(&*mem_space)?.ok_or_else(|| errno!(EFAULT))?;
		(Path::from_str(path, true)?, proc.access_profile)
	};

	let file_mutex = vfs::get_file_from_path(&path, &ap, follow_links)?;
	let mut file = file_mutex.lock();
	// TODO allow changing group to any group whose owner is member
	if !ap.is_privileged() {
		return Err(errno!(EPERM));
	}
	if owner != -1 {
		file.set_uid(owner as _);
	}
	if group != -1 {
		file.set_gid(group as _);
	}
	// TODO lazy
	file.sync()?;

	Ok(0)
}

#[syscall]
pub fn chown(pathname: SyscallString, owner: c_int, group: c_int) -> EResult<i32> {
	do_chown(pathname, owner, group, true)
}
