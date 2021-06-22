//! This module handles CPU-related features, including multicore.
//!
//! Before booting a new CPU core, the kernel relocates what's called the "trampoline code" which
//! is the code to be executed by the new core in the beginning. Since the core is booting in real
//! mode, the trampoline code is required to be assembly code.

use core::ffi::c_void;
use core::mem::transmute;
use core::ptr;
use crate::errno::Errno;
use crate::memory;
use crate::time;
use crate::util::container::vec::Vec;
use crate::util::lock::mutex::Mutex;
use crate::util;

mod startup;
pub mod pic;

/// The physical address to the destination of the trampoline.
const TRAMPOLINE_PTR: *mut c_void = 0x8000 as *mut c_void;
/// The size of the trampoline code in bytes. This value can be a bit larger than required.
const TRAMPOLINE_SIZE: usize = memory::PAGE_SIZE;

/// The offset of the APIC error status register.
const APIC_OFFSET_ERROR_STATUS: usize = 0x280;
/// The offset of the APIC Interrupt Command Register register 0.
const APIC_OFFSET_ICR0: usize = 0x300;
/// The offset of the APIC Interrupt Command Register register 1.
const APIC_OFFSET_ICR1: usize = 0x310;

extern "C" {
	fn msr_exist() -> u32;
	fn msr_read(i: u32, lo: *mut u32, hi: *mut u32);
	fn msr_write(i: u32, lo: u32, hi: u32);

	fn get_current_apic() -> u32;

	fn cpu_trampoline();
}

/// Model Specific Register (MSR) features.
pub mod msr {
	use super::*;

	/// Tells whether MSR exist.
	pub fn exist() -> bool {
		unsafe {
			msr_exist() != 0
		}
	}

	/// Reads the `i`th MSR and returns its value.
	pub fn read(i: u32) -> u64 {
		let mut lo = 0;
		let mut hi = 0;
		unsafe {
			msr_read(i, &mut lo, &mut hi);
		}

		((hi as u64) << 32) | (lo as u64)
	}

	/// Writes the `i`th MSR with value `val`.
	pub fn write(i: u32, val: u64) {
		unsafe {
			msr_write(i, (val & 0xffff) as _, ((val >> 32) & 0xffff) as _);
		}
	}
}

/// Module handling APIC-related features.
pub mod apic {
	use super::*;

	/// The APIC's virtual address.
	static mut APIC_ADDR: Option<*mut c_void> = None;

	/// Sets the APIC physical address.
	/// This function is **not** thread-safe.
	pub unsafe fn set_addr(addr: *mut c_void) {
		// TODO Remap kernel? (since the APIC seems to be accessed through DMA)
		APIC_ADDR = Some(memory::kern_to_virt(addr) as _);
	}

	/// Enables the APIC.
	/// This function requires the APIC address to be set first. If not set, the behaviour is
	/// undefined.
	/// This function is **not** thread-safe.
	pub fn enable() {
		// TODO
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
		let ptr = (APIC_ADDR.unwrap() as usize + offset) as *mut u32;
		debug_assert!(util::is_aligned(ptr as _, 16));
		ptr
	}

	/// Waits until the interrupt has been delivered.
	/// This function requires the APIC address to be set first. If not set, the behaviour is
	/// undefined.
	pub fn wait_delivery() {
		unsafe { // Safe because the register offset is valid
			let icr0 = apic::get_register(APIC_OFFSET_ICR0);
			while ptr::read_volatile(icr0) & (1 << 12) != 0 {}
		}
	}

	/// Sends an End-Of-Interrupt message to the APIC for the given interrupt `irq`.
	pub fn end_of_interrupt(_irq: u8) {
		// TODO
	}
}

/// Structure representing a CPU core.
pub struct CPU {
	/// The CPU ID.
	id: u32,
	/// The APIC ID.
	apic_id: u32,
	/// The I/O APIC address.
	io_apic_addr: Option<*mut u32>,

	/// CPU flags.
	flags: u32,
}

impl CPU {
	/// Creates a new instance.
	pub fn new(id: u32, apic_id: u32, flags: u32) -> Self {
		Self {
			id,
			apic_id,
			io_apic_addr: None,

			flags,
		}
	}

	/// Returns the CPU's ID.
	pub fn get_id(&self) -> u32 {
		self.id
	}

	/// Returns the APIC ID.
	pub fn get_apic_id(&self) -> u32 {
		self.apic_id
	}

	/// Returns the I/O APIC physical address.
	pub fn get_io_apic_addr(&self) -> Option<*mut u32> {
		self.io_apic_addr
	}

	/// Sets the I/O APIC physical address.
	pub fn set_io_apic_addr(&mut self, addr: Option<*mut u32>) {
		self.io_apic_addr = addr;
	}

	/// Returns CPU's APIC flags.
	pub fn get_flags(&self) -> u32 {
		self.flags
	}

	/// Tells whether the CPU can be enabled.
	pub fn can_enable(&self) -> bool {
		(self.flags & 0b1 != 0) || (self.flags & 0b10 != 0)
	}

	/// Enables the CPU. If already enabled, the behaviour is undefined.
	pub fn enable(&self) {
		unsafe {
			let err = apic::get_register(APIC_OFFSET_ERROR_STATUS);
			let icr0 = apic::get_register(APIC_OFFSET_ICR0);
			let icr1 = apic::get_register(APIC_OFFSET_ICR1);

			ptr::write_volatile(err, 0);

			ptr::write_volatile(icr1, (ptr::read_volatile(icr1) & 0x00ffffff)
				| (self.apic_id << 24));
			ptr::write_volatile(icr0, (ptr::read_volatile(icr0) & 0xfff00000) | 0xc500);
			apic::wait_delivery();

			ptr::write_volatile(icr1, (ptr::read_volatile(icr1) & 0x00ffffff)
				| (self.apic_id << 24));
			ptr::write_volatile(icr0, (ptr::read_volatile(icr0) & 0xfff00000) | 0x8500);
			apic::wait_delivery();

			time::mdelay(10);

			for _ in 0..2 {
				ptr::write_volatile(err, 0);

				ptr::write_volatile(icr1, (ptr::read_volatile(icr1) & 0x00ffffff)
					| (self.apic_id << 24));
				ptr::write_volatile(icr0, (ptr::read_volatile(icr0) & 0xfff0f800) | 0x000608);

				time::udelay(200);
				apic::wait_delivery();
			}
		}
	}
}

/// The list of CPUs on the system.
static mut CPUS: Mutex<Vec<Mutex<CPU>>> = Mutex::new(Vec::new());

/// Returns the number of CPUs on the system.
pub fn get_count() -> usize {
	unsafe { // Safe because using Mutex
		CPUS.lock().get().len()
	}
}

// TODO Return a dynamicaly associated ID instead to ensure that the IDs are linear
/// Returns the ID of the current running CPU.
pub fn get_current() -> u32 {
	unsafe {
		get_current_apic()
	}
}

/// Adds a new core to the core list.
pub fn add_core(cpu: CPU) -> Result<(), Errno> {
	unsafe { // Safe because using Mutex
		CPUS.lock().get_mut().push(Mutex::new(cpu))
	}
}

/// Returns a mutable reference to the CPUs list's Mutex.
pub fn list() -> &'static mut Mutex<Vec<Mutex<CPU>>> {
	unsafe {
		&mut CPUS
	}
}

/// Copies the trampoline code to its destination address to ensure it is accessible from real mode
/// CPUs.
fn relocate_trampoline() {
	let src = unsafe {
		transmute::<unsafe extern "C" fn(), *const c_void>(cpu_trampoline)
	};
	let dest = memory::kern_to_virt(TRAMPOLINE_PTR) as _;

	unsafe {
		ptr::copy_nonoverlapping(src, dest, TRAMPOLINE_SIZE);
	}
}

/// Initializes CPU cores other than the main core.
/// This function also disables the PIC.
/// This function must be called **only once, at boot**.
pub fn init_multicore() {
	pic::disable();
	apic::enable();

	relocate_trampoline();

	let curr_id = get_current();
	let mut cores_guard = unsafe { // Safe because using Mutex
		CPUS.lock()
	};
	let cores = cores_guard.get_mut();
	for i in 0..cores.len() {
		let cpu_guard = cores[i].lock();
		let cpu = cpu_guard.get();

		if cpu.apic_id != curr_id && cpu.can_enable() {
			cpu.enable();
		}
	}
}

/// The function to be called at the end of an interrupt.
#[no_mangle]
extern "C" fn end_of_interrupt(irq: u8) {
	let enabled = unsafe {
		apic::is_enabled()
	};

	if enabled {
		apic::end_of_interrupt(irq);
	} else {
		pic::end_of_interrupt(irq);
	}
}
