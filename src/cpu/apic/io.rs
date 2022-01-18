//! The I/O APIC allows to control interruptions coming from outside of the CPU.

use core::ptr;

/// Structure representing an I/O APIC.
pub struct IOAPIC {
	/// The base physical address for the I/O APIC's registers.
	regs_base: *mut u32,
}

impl IOAPIC {
	/// Reads the given register and returns its value.
	/// `offset` is the offset of the register.
	pub fn reg_read(&self, offset: usize) -> u32 {
		unsafe {
			ptr::write_volatile(self.regs_base, offset as _);
			ptr::read_volatile(self.regs_base.add(4))
		}
	}

	/// Writes the given register with the given value.
	/// `offset` is the offset of the register.
	/// `value` is the value to write.
	pub fn reg_write(&self, offset: usize, value: u32) {
		unsafe {
			ptr::write_volatile(self.regs_base, offset as _);
			ptr::write_volatile(self.regs_base.add(4), value);
		}
	}

	/// Sets the `n`th entry of the I/O APIC.
	/// `v` is the vector for the interrupt. Valid values are between 0x10 and 0xfe.
	pub fn set_entry(&self, _n: usize, _v: u8) {
		// TODO
		todo!();
	}
}
