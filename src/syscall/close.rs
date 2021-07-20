//! TODO doc

use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::util;

/// The implementation of the `close` syscall.
pub fn close(regs: &util::Regs) -> Result<i32, Errno> {
	let fd = regs.ebx;

	let mut mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock(false);
	let proc = guard.get_mut();

	if proc.close_fd(fd).is_ok() {
		Ok(0)
	} else {
		Err(errno::EBADF)
	}
}
