//! This module handles CPU-related features, including multicore.

use core::ffi::c_void;
use core::ptr::null_mut;
use crate::errno::Errno;
use crate::memory;
use crate::util::container::vec::Vec;
use crate::util::lock::mutex::Mutex;

extern "C" {
	fn get_current_apic() -> u32;
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

/// Returns the ID of the current running APIC.
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

/// Initializes CPU cores other than the main core.
/// This function must be called **only once, at boot**.
pub fn init_multicore() {
	let mut cores_guard = unsafe { // Safe because using Mutex
		CPUS.lock()
	};
	let cores = cores_guard.get_mut();

	let curr_id = get_current();
	for i in 0..cores.len() {
		let cpu_guard = cores[i].lock();
		let cpu = cpu_guard.get();

		if cpu.apic_id == curr_id {
			continue;
		}

		// TODO
	}
}
