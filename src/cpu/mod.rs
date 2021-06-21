//! This module handles CPU-related features, including multicore.
//!
//! Before booting a new CPU core, the kernel relocates what's called the "trampoline code" which
//! is the code to be executed by the new core in the beginning. Since the core is booting in real
//! mode, the trampoline code is required to be assembly code.

use core::ffi::c_void;
use core::mem::transmute;
use core::ptr::null_mut;
use core::ptr;
use crate::errno::Errno;
use crate::memory;
use crate::util::container::vec::Vec;
use crate::util::lock::mutex::Mutex;

/// The physical address to the destination of the trampoline.
const TRAMPOLINE_PTR: *mut c_void = 0x8000 as *mut c_void;
/// The size of the trampoline code in bytes. This value can be a bit larger than required.
const TRAMPOLINE_SIZE: usize = memory::PAGE_SIZE;

extern "C" {
	fn get_current_apic() -> u32;

	fn cpu_trampoline();
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
}

/// The APIC's virtual address.
static mut APIC_ADDR: *mut c_void = null_mut();
/// The list of CPUs on the system.
static mut CPUS: Mutex<Vec<Mutex<CPU>>> = Mutex::new(Vec::new());

/// Sets the APIC physical address.
/// This function is **not** thread-safe.
pub unsafe fn set_apic_addr(addr: *mut c_void) {
	// TODO Remap kernel? (since the APIC seems to be accessed through DMA)
	APIC_ADDR = memory::kern_to_virt(addr) as _;
}

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
/// This function must be called **only once, at boot**.
pub fn init_multicore() {
	relocate_trampoline();

	let curr_id = get_current();
	let mut cores_guard = unsafe { // Safe because using Mutex
		CPUS.lock()
	};
	let cores = cores_guard.get_mut();
	for i in 0..cores.len() {
		let cpu_guard = cores[i].lock();
		let cpu = cpu_guard.get();

		if cpu.apic_id == curr_id || !cpu.can_enable() {
			continue;
		}

		// TODO Enable the CPU
	}
}
