//! Model Specific Register (MSR) features.

/// The MSR for the local APIC base address.
pub const APIC_BASE: u32 = 0x1b;

extern "C" {
	fn msr_exist() -> u32;
	fn msr_read(i: u32, lo: *mut u32, hi: *mut u32);
	fn msr_write(i: u32, lo: u32, hi: u32);
}

/// Tells whether the MSR exist.
pub fn exist() -> bool {
	unsafe {
		msr_exist() != 0
	}
}

/// Reads the `i`th MSR and writes its value into `lo` and `hi`.
pub fn read(i: u32, lo: &mut u32, hi: &mut u32) {
	unsafe {
		msr_read(i, lo, hi);
	}
}

/// Writes the `i`th MSR with values `lo` and `hi`.
pub fn write(i: u32, lo: u32, hi: u32) {
	unsafe {
		msr_write(i, lo, hi);
	}
}
