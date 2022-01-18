//! The _exit syscall allows to terminate the current process with the given status code.

use core::arch::asm;
use crate::process::Process;
use crate::process::regs::Regs;

/// The implementation of the `write` syscall.
pub fn _exit(regs: &Regs) -> ! {
	{
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock();
		let proc = guard.get_mut();

		proc.exit(regs.ebx);
	}

	// TODO Make sure this is valid (probably not) since the stack is not accessible anymore
	unsafe {
		// Waiting for the next tick
		asm!("jmp $cpu_loop");
	}

	unreachable!();
}
