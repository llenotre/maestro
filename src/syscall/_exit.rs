/// TODO doc

use crate::process::Process;
use crate::process::tss;
use crate::util;

/// The implementation of the `write` syscall.
pub fn _exit(regs: &util::Regs) -> ! {
	{
		Process::get_current().unwrap().lock().get().exit(regs.eax);
	}

	// TODO Fix: The stack might be removed while being used (example: process is
	// killed, its exit status is retrieved from another CPU core and then the process
	// is removed)
	unsafe {
		crate::kernel_loop_reset(tss::get().esp0 as _);
	}
}
