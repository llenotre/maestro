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

//! The `fstat64` system call allows get the status of a file.

use crate::{
	file::{
		perm::{Gid, Uid},
		INode, Mode,
	},
	process::{mem_space::ptr::SyscallPtr, Process},
	time::unit::{TimeUnit, Timespec, TimestampScale},
};
use core::ffi::{c_int, c_long};
use macros::syscall;
use utils::{errno, errno::Errno, io::IO};

// TODO Check types
/// Structure containing the informations of a file.
#[repr(C)]
#[derive(Debug)]
struct Stat {
	/// ID of the device containing the file.
	st_dev: u64,

	/// Padding.
	__st_dev_padding: c_int,

	/// The inode number.
	st_ino: INode,
	/// File's mode.
	st_mode: Mode,
	/// Number of hard links to the file.
	st_nlink: u32,
	/// File's owner UID.
	st_uid: Uid,
	/// File's owner GID.
	st_gid: Gid,
	/// Device ID (if device file).
	st_rdev: u64,

	/// Padding.
	__st_rdev_padding: c_int,

	/// Size of the file in bytes.
	st_size: u32,
	/// Size of a block on the file's storage medium.
	st_blksize: c_long,
	/// Size of the file in blocks.
	st_blocks: u64,

	/// Timestamp of last access.
	st_atim: Timespec,
	/// Timestamp of last modification of the content.
	st_mtim: Timespec,
	/// Timestamp of last modification of the metadata.
	st_ctim: Timespec,
}

#[syscall]
pub fn fstat64(fd: c_int, statbuf: SyscallPtr<Stat>) -> Result<i32, Errno> {
	if fd < 0 {
		return Err(errno!(EBADF));
	}

	let open_file_mutex = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let fds_mutex = proc.file_descriptors.as_ref().unwrap();
		let fds = fds_mutex.lock();

		fds.get_fd(fd as _)
			.ok_or_else(|| errno!(EBADF))?
			.get_open_file()
			.clone()
	};
	let open_file = open_file_mutex.lock();

	let file_mutex = open_file.get_file();
	let file = file_mutex.lock();

	let inode = file.get_location().get_inode();

	let stat = Stat {
		st_dev: 0, // TODO

		__st_dev_padding: 0,

		st_ino: inode,
		st_mode: file.get_mode(),
		st_nlink: file.get_hard_links_count() as _,
		st_uid: file.get_uid(),
		st_gid: file.get_gid(),
		st_rdev: 0, // TODO

		__st_rdev_padding: 0,

		st_size: file.get_size() as _,
		st_blksize: 512, // TODO
		st_blocks: file.blocks_count,

		st_atim: Timespec::from_nano(TimestampScale::convert(
			file.atime,
			TimestampScale::Second,
			TimestampScale::Nanosecond,
		)),
		st_mtim: Timespec::from_nano(TimestampScale::convert(
			file.mtime,
			TimestampScale::Second,
			TimestampScale::Nanosecond,
		)),
		st_ctim: Timespec::from_nano(TimestampScale::convert(
			file.ctime,
			TimestampScale::Second,
			TimestampScale::Nanosecond,
		)),
	};

	{
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mut mem_space_guard = mem_space.lock();

		let statbuf = statbuf
			.get_mut(&mut mem_space_guard)?
			.ok_or(errno!(EFAULT))?;
		*statbuf = stat;
	}

	Ok(0)
}
