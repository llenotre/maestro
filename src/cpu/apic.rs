//! The Advanced Programmable Interrupt Controller (APIC) is the successor of the PIC. It is meant
/// to support multicore CPUs.

use core::arch::asm;
use core::ptr;
use crate::cpu::msr;
use crate::cpu;
use crate::memory;
use crate::util;

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

/// Structure representing an APIC.
pub struct APIC {
	/// The APIC's ID.
	id: u8,
	/// Tells whether the APIC is enabled.
	enabled: bool,

	/// The base physical address for the APIC's registers.
	regs_base: *mut u32,
}

impl APIC {
	/// Reads the base address of the local APIC from the MSR and returns it.
	fn get_apic_base() -> *mut u32 {
		let mut lo = 0;
		let mut hi = 0;
		msr::read(msr::APIC_BASE, &mut lo, &mut hi);

		((lo as u64 & 0xfffff000) | ((hi as u64 & 0xf) << 32)) as _
	}

	/// Returns a mutable reference to the APIC register at offset `offset`.
	/// If the offset is invalid, the behaviour is undefined.
	fn get_reg(&self, offset: usize) -> *mut u32 {
		let ptr = (memory::kern_to_virt(self.regs_base as _) as usize + offset) as *mut u32;
		debug_assert!(util::is_aligned(ptr, 16));

		ptr
	}

	/// Reads the given register and returns its value.
	/// `offset` is the offset of the register.
	pub fn reg_read(&self, offset: usize) -> u32 {
		let reg_ptr = self.get_reg(offset);

		unsafe {
			ptr::read_volatile(reg_ptr)
		}
	}

	/// Writes the given register with the given value.
	/// `offset` is the offset of the register.
	/// `value` is the value to write.
	pub fn reg_write(&self, offset: usize, value: u32) {
		let reg_ptr = self.get_reg(offset);

		unsafe {
			ptr::write_volatile(reg_ptr, value);
		}
	}

	/// Creates a new instance.
	/// `id` is the APIC's ID.
	pub fn new(id: u8) -> Self {
		Self {
			id,
			enabled: false,

			regs_base: 0x0 as _,
		}
	}

	/// Returns the APIC's ID.
	pub fn get_id(&self) -> u8 {
		self.id
	}

	/// Tells whether the APIC is enabled.
	pub fn is_enabled(&self) -> bool {
		self.enabled
	}

	/// Enables the APIC. If already enabled, the function does nothing.
	/// This function works only for the current core. TODO doc
	pub fn enable(&mut self) {
		if self.enabled {
			return;
		}

		// Check the APIC is the current core's APIC
		if self.get_id() != cpu::get_current_id() {
			// TODO Panic?
		}

		// Getting the registers' base
		let base = Self::get_apic_base();
		self.regs_base = base;

		// Enabling APIC
		msr::write(msr::APIC_BASE, ((base as u32) & 0xffff0000) | 0x800, 0);

		// Sets the Spurious Interrupt Vector bit 8 to enable receiving interrupts
		self.reg_write(REG_OFFSET_SIV, self.reg_read(REG_OFFSET_SIV) | 0x100);

		self.enabled = true;
	}

	/// Waits until the interrupt has been delivered.
	/// This function requires the APIC address to be set first. If not set, the behaviour is
	/// undefined.
	pub fn wait_delivery(&self) {
		while self.reg_read(REG_OFFSET_ICR0) & (1 << 12) != 0 {
			unsafe {
				asm!("pause");
			}
		}
	}

	/// Sends an End-Of-Interrupt message to the APIC for the given interrupt `irq`.
	pub fn end_of_interrupt(&self, _irq: u8) {
		self.reg_write(REG_OFFSET_EOI, 0);
	}
}
