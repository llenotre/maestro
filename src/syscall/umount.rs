//! The `umount` system call allows to unmount a filesystem previously mounted
//! with `mount`.

use crate::errno;
use crate::errno::Errno;
use crate::file::path::Path;
use crate::file::vfs::ResolutionSettings;
use crate::file::{mountpoint, vfs};
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn umount(target: SyscallString) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	// Check permission
	if !proc.access_profile.is_privileged() {
		return Err(errno!(EPERM));
	}

	let rs = ResolutionSettings::for_process(&proc, true);

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();

	// Get target directory
	let target_slice = target.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;
	let target_path = Path::new(target_slice)?;
	let target_dir = vfs::get_file_from_path(target_path, &rs)?;
	let target_dir = target_dir.lock();

	// Remove mountpoint
	mountpoint::remove(target_dir.get_location())?;

	Ok(0)
}
