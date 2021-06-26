//! This module implements the function to be called when a new CPU (other than the main one)
//! starts.

use crate::cpu;
use crate::idt;

/// This function is called for each CPU that starts.
#[no_mangle]
extern "C" fn cpu_startup() -> ! {
	idt::bind();
	// TODO Bind the kernel virtual memory context handler to the current CPU

	crate::println!("Hello from CPU {}", cpu::get_current()); // TODO rm
	crate::halt(); // TODO rm

	// TODO Wait until the scheduler is ready, then enable interrupts to execute processes
}
