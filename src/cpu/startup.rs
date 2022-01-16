//! This module implements the function to be called when a new CPU (other than the main one)
//! starts.

use crate::cpu;
use crate::idt;

/// This function is called for each CPU that starts.
#[no_mangle]
extern "C" fn cpu_startup() -> ! {
	idt::bind();
	// TODO Enable local APIC

	crate::println!("Hello from CPU {}", cpu::get_current_id()); // TODO rm
	cpu::halt(); // TODO rm

	// TODO Wait until the scheduler is ready, then enable interrupts to execute processes
}
