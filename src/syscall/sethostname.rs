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

//! The `sethostname` syscall sets the hostname of the system.

use crate::errno::Errno;
use crate::limits;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn sethostname(name: SyscallSlice<u8>, len: usize) -> Result<i32, Errno> {
	// Check the size of the hostname is in bounds
	if len > limits::HOST_NAME_MAX {
		return Err(errno!(EINVAL));
	}

	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	// Checking permission
	if !proc.access_profile.is_privileged() {
		return Err(errno!(EPERM));
	}

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();
	let name_slice = name.get(&mem_space_guard, len)?.ok_or(errno!(EFAULT))?;

	let mut hostname = crate::HOSTNAME.lock();
	hostname.resize(len)?;
	hostname.as_mut_slice().copy_from_slice(name_slice);

	Ok(0)
}
