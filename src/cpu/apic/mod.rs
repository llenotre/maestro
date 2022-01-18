//! The Advanced Programmable Interrupt Controller (APIC) is the successor of the PIC. It is meant
/// to support multicore CPUs.

mod io;
mod timer;

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
/// The offset of the APIC LVT Timer Register.
pub const REG_OFFSET_LVT_TIMER: usize = 0x320;
/// The offset of the APIC Initial Count Register.
pub const REG_OFFSET_ICR: usize = 0x380;
/// The offset of the APIC Divide Configuration Register.
pub const REG_OFFSET_DCR: usize = 0x3e0;

/// Enumeration representing the destination of an IPI.
pub enum IPIDest {
	/// The destination is the given APIC ID.
	Number(u8),
	/// The destination is the current CPU.
	SelfCPU,
	/// The IPI is sent to every CPUs.
	AllIncludingSelf,
	/// The IPI is sent to every CPUs except the current one.
	AllExcludingSelf,
}

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
	/// If this function is called for a core which is not the current one, it does nothing.
	pub fn enable(&mut self) {
		if self.enabled {
			return;
		}

		// Check the APIC is the current core's APIC
		if self.get_id() != cpu::get_current_id() {
			return;
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

	/// Waits until data written in a register has been delivered.
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

	// TODO Allow startup interrupt
	/// Sends an Inter-Processor Interrupt with vector `n` to the given destination `dest`.
	/// The function waits until the interrupt is delivered.
	pub fn send_ipi(&self, n: u8, dest: IPIDest) {
		let apic_id = match dest {
			IPIDest::Number(n) => n,
			_ => 0,
		};

		let dest_shorthand = match dest {
			IPIDest::Number(_) => 0b00,
			IPIDest::SelfCPU => 0b01,
			IPIDest::AllIncludingSelf => 0b10,
			IPIDest::AllExcludingSelf => 0b11,
		};

		let icr1 = (self.reg_read(REG_OFFSET_ICR1) & 0x00ffffff) | ((apic_id as u32) << 24);
		self.reg_write(REG_OFFSET_ICR1, icr1);

		let icr0 = (self.reg_read(REG_OFFSET_ICR0) & 0xfff00000) | (dest_shorthand << 18)
			| (n as u32);
		self.reg_write(REG_OFFSET_ICR0, icr0);

		self.wait_delivery();
	}
}
