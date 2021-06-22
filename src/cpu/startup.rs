//! This module implements the function to be called when a new CPU (other than the main one) starts.

use crate::cpu;

/// This function is called for each CPU that start.
#[no_mangle]
extern "C" fn cpu_startup() -> ! {
	crate::println!("Hello from CPU {}", cpu::get_current()); // TODO rm
	// TODO Wait until the scheduler is ready, then enable interrupts to execute processes

	crate::halt();
}
