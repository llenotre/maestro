//! This module handles CPU-related features, including multicore.
//!
//! Before booting a new CPU core, the kernel relocates what's called the "trampoline code" which
//! is the code to be executed by the new core in the beginning. Since the core is booting in real
//! mode, the trampoline code is required to be assembly code.

mod startup;
pub mod apic;
pub mod pic;
pub mod sse;

use core::ffi::c_void;
use core::mem::size_of;
use core::mem::transmute;
use core::ptr;
use crate::errno::Errno;
use crate::memory::malloc;
use crate::memory::vmem;
use crate::memory;
use crate::time;
use crate::util::container::vec::Vec;
use crate::util::lock::Mutex;

/// The physical address to the destination of the trampoline.
const TRAMPOLINE_PTR: *mut c_void = 0x8000 as *mut c_void;
/// The size of the trampoline code in bytes. This value can be a bit larger than required.
const TRAMPOLINE_SIZE: usize = memory::PAGE_SIZE;
/// The size of a core's startup stack.
const CORE_STACK_SIZE: usize = memory::PAGE_SIZE * 8;

extern "C" {
	fn get_current_apic() -> u32;

	fn cpu_trampoline();
	static mut trampoline_stacks: u32;

	/// Tells whether the CPU has SSE.
	fn cpuid_has_sse() -> bool;

	/// Returns the content of the %cr0 register.
	pub fn cr0_get() -> u32;
	/// Sets the given flags in the %cr0 register.
	pub fn cr0_set(flags: u32);
	/// Clears the given flags in the %cr0 register.
	pub fn cr0_clear(flags: u32);
	/// Returns the content of the %cr2 register.
	pub fn cr2_get() -> *const c_void;
	/// Returns the content of the %cr3 register.
	pub fn cr3_get() -> *mut c_void;
	/// Returns the content of the %cr4 register.
	pub fn cr4_get() -> u32;
	/// Sets the content of the %cr4 register.
	pub fn cr4_set(flags: u32);
}

/// Model Specific Register (MSR) features.
pub mod msr {
    extern "C" {
        fn msr_exist() -> u32;
        fn msr_read(i: u32, lo: *mut u32, hi: *mut u32);
        fn msr_write(i: u32, lo: u32, hi: u32);
    }

	/// Tells whether MSR exist.
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
			let err = apic::get_register(apic::REG_OFFSET_ERROR_STATUS);
			let icr0 = apic::get_register(apic::REG_OFFSET_ICR0);
			let icr1 = apic::get_register(apic::REG_OFFSET_ICR1);

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
/// The function also allocates stacks for each cores.
/// `cores_count` is the number of cores on the system.
fn prepare_trampoline(cores_count: usize) -> Result<(), Errno> {
	let stacks = unsafe {
		malloc::alloc(cores_count * size_of::<u32>())? as *mut u32
	};
	for i in 0..cores_count {
		let res = unsafe {
			malloc::alloc(CORE_STACK_SIZE)
		};

		if let Ok(ptr) = res {
			unsafe {
				*stacks.add(i) = ptr as _;
			}
		} else {
			for j in 0..i {
				unsafe {
					malloc::free(*stacks.add(j) as _);
				}
			}

			unsafe {
				malloc::free(stacks as _);
			}
			return Err(res.unwrap_err());
		}
	}

	let src = unsafe {
		transmute::<unsafe extern "C" fn(), *const c_void>(cpu_trampoline)
	};
	let dest = memory::kern_to_virt(TRAMPOLINE_PTR) as _;

	unsafe {
		vmem::write_lock_wrap(|| {
			ptr::copy_nonoverlapping(src, dest, TRAMPOLINE_SIZE);

			// TODO Free when every cores are ready?
			trampoline_stacks = stacks as u32;
		});
	}

	Ok(())
}

/// Initializes CPU cores other than the main core.
/// If more than one CPU core is present on the system, the PIC is disabled and the APIC is enabled
/// instead.
/// This function must be called **only once, at boot**.
pub fn init_multicore() {
	let curr_id = get_current();
	let mut cores_guard = unsafe { // Safe because using Mutex
		CPUS.lock()
	};
	let cores = cores_guard.get_mut();
	let cores_count = cores.len();

	if cores_count > 1 {
		pic::disable();
		apic::enable();

		if prepare_trampoline(cores_count).is_err() {
			crate::kernel_panic!("Failed to initialize multicore");
		}

		for i in 0..cores_count {
			let cpu_guard = cores[i].lock();
			let cpu = cpu_guard.get();

			if cpu.apic_id != curr_id && cpu.can_enable() {
				cpu.enable();
			}
		}
	}
}

/// The function to be called at the end of an interrupt.
#[no_mangle]
pub extern "C" fn end_of_interrupt(irq: u8) {
	let apic_enabled = unsafe {
		apic::is_enabled()
	};

	if apic_enabled {
		apic::end_of_interrupt(irq);
	} else {
		pic::end_of_interrupt(irq);
	}
}
