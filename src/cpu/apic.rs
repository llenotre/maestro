//! The APIC is the successor of the PIC. It is meant to support multicore CPUs.

use core::ffi::c_void;
use core::ptr;
use crate::cpu::msr;
use crate::memory;
use crate::util;

/// The APIC's physical address.
static mut APIC_ADDR: Option<*mut c_void> = None;

/// The offset of the APIC End Of Interrupt register.
pub const REG_OFFSET_EOI: usize = 0xb0;
/// The offset of the APIC Spurious Interrupt Vector register.
pub const REG_OFFSET_SIV: usize = 0xf0;
/// The offset of the APIC error status register.
pub const REG_OFFSET_ERROR_STATUS: usize = 0x280;
/// The offset of the APIC Interrupt Command Register register 0.
pub const REG_OFFSET_ICR0: usize = 0x300;
/// The offset of the APIC Interrupt Command Register register 1.
pub const REG_OFFSET_ICR1: usize = 0x310;

/// Sets the APIC physical address.
/// This function is **not** thread-safe.
pub unsafe fn set_addr(addr: *mut c_void) {
	APIC_ADDR = Some(addr);
}

/// Enables the APIC.
/// This function requires the APIC address to be set first. If not set, the behaviour is
/// undefined.
/// This function is **not** thread-safe.
pub fn enable() {
	let mut lo = 0;
	let mut hi = 0;
	// TODO Place constant values into constants
	msr::read(0x1b, &mut lo, &mut hi);
	msr::write(0x1b, (lo & 0xffff0000) | 0x800, 0);

	unsafe {
		let siv = get_register(REG_OFFSET_SIV);
		ptr::write_volatile(siv, ptr::read_volatile(siv) | 0x100);
	}
}

/// Tells whether the APIC is enabled.
/// This function is **not** thread-safe.
pub unsafe fn is_enabled() -> bool {
	APIC_ADDR.is_some()
}

/// Returns a mutable reference to the APIC register at offset `offset`.
/// This function requires the APIC address to be set first. If not set, the behaviour is
/// undefined.
/// If the offset is invalid, the behaviour is undefined.
pub unsafe fn get_register(offset: usize) -> *mut u32 {
	let ptr = (memory::kern_to_virt(APIC_ADDR.unwrap()) as usize + offset) as *mut u32;
	debug_assert!(util::is_aligned(ptr as _, 16));
	ptr
}

/// Waits until the interrupt has been delivered.
/// This function requires the APIC address to be set first. If not set, the behaviour is
/// undefined.
pub fn wait_delivery() {
	unsafe { // Safe because the register offset is valid
		let icr0 = get_register(REG_OFFSET_ICR0);
		while ptr::read_volatile(icr0) & (1 << 12) != 0 {}
	}
}

/// Sends an End-Of-Interrupt message to the APIC for the given interrupt `irq`.
pub fn end_of_interrupt(_irq: u8) {
	unsafe { // Safe because the register offset is valid
		let eoi = get_register(REG_OFFSET_EOI);
		ptr::write_volatile(eoi, 0);
	}
}
