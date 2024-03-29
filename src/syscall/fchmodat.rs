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

//! The `fchmodat` system call allows change the permissions on a file.

use super::util;
use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;

// TODO Check args type
#[syscall]
pub fn fchmodat(
	dirfd: c_int,
	pathname: SyscallString,
	mode: i32,
	flags: c_int,
) -> Result<i32, Errno> {
	let (file_mutex, ap) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let ap = proc.access_profile;

		let mem_space = proc.get_mem_space().unwrap().clone();
		let mem_space_guard = mem_space.lock();

		let pathname = pathname
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let file_mutex = util::get_file_at(proc, dirfd, pathname, true, flags)?;

		(file_mutex, ap)
	};
	let mut file = file_mutex.lock();

	// Check permissions
	if !ap.can_set_file_permissions(&*file) {
		return Err(errno!(EPERM));
	}

	file.set_permissions(mode as _);
	// TODO lazy sync
	file.sync()?;

	Ok(0)
}
