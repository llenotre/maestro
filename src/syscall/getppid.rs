//! TODO doc

use crate::errno::Errno;
use crate::process::Process;
use crate::util;

/// The implementation of the `getppid` syscall.
pub fn getppid(_regs: &util::Regs) -> Result<i32, Errno> {
	let mut mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock(false);
	let proc = guard.get_mut();

	Ok(proc.get_parent_pid() as _)
}
