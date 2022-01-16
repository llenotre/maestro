//! This module handles CPU-related features, including multicore.
//!
//! Before booting a new CPU core, the kernel relocates what's called the "trampoline code" which
//! is the code to be executed by the new core in the beginning. Since the core is booting in real
//! mode, the trampoline code is required to be assembly code.

mod startup;
pub mod apic;
pub mod pic;
pub mod sse;

use apic::APIC;
use core::ffi::c_void;
use core::mem::transmute;
use core::ptr::NonNull;
use core::ptr;
use crate::errno::Errno;
use crate::memory::malloc;
use crate::memory::vmem;
use crate::memory;
use crate::time;
use crate::util::container::vec::Vec;
use crate::util::lock::IntMutex;
use crate::util::lock::Mutex;
use crate::util::math;

/// The physical address to the destination of the trampoline.
const TRAMPOLINE_PTR: *mut c_void = 0x8000 as *mut c_void;
/// The size of a core's startup stack.
const CORE_STACK_SIZE: usize = memory::PAGE_SIZE * 8;

extern "C" {
	/// Returns the ID of the current APIC.
	fn get_current_apic() -> u32;
	/// Tells whether the CPU has SSE.
	fn cpuid_has_sse() -> bool;
	/// Returns the core crystal clock frequency in `clock_freq` and the ratio between the clock
	/// and the TSC in `tsc_numerator` and `tsc_denominator`.
	fn cpuid_clock_ratios(tsc_denominator: &mut u32, tsc_numerator: &mut u32,
		clock_freq: &mut u32);

	/// The symbol of the CPU's startup trampoline.
	fn cpu_trampoline();
	/// The pointer to the trampoline stacks.
	static mut trampoline_stacks: *const *mut u8;
	/// The pointer to the kernel vmem to use in the trampoline
	static mut trampoline_vmem: *mut u32;
	/// The symbol at the end of the trampoline.
	static trampoline_end: c_void;

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

	fn cpu_wait();
	fn cpu_loop() -> !;
	fn cpu_loop_reset(stack: *mut c_void) -> !;
	fn cpu_halt() -> !;
}

/// Makes the current CPU wait for an interrupt, processes it, then returns.
/// This function enables interrupts.
pub fn wait() {
	unsafe {
		cpu_wait();
	}
}

/// Makes the current CPU loop and process every interrupts indefinitely.
pub fn enter_loop() -> ! {
	unsafe {
		cpu_loop();
	}
}

/// Resets the stack to the given value, then calls `enter_loop`.
/// The function is marked as unsafe because the pointer passed as parameter might be invalid.
pub unsafe fn loop_reset(stack: *mut c_void) -> ! {
	cpu_loop_reset(stack);
}

/// Halts the current CPU until reboot.
pub fn halt() -> ! {
	unsafe {
		cpu_halt();
	}
}

/// Model Specific Register (MSR) features.
pub mod msr {
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
}

/// Structure representing a CPU core.
pub struct CPU {
	/// The CPU ID.
	id: u32,
	/// The CPU's APIC.
	apic: APIC,
	/// The I/O APIC address.
	io_apic_addr: Option<NonNull<u32>>,

	/// CPU flags.
	flags: u32,
}

impl CPU {
	/// Creates a new instance.
	pub fn new(id: u32, apic: APIC, flags: u32) -> Self {
		Self {
			id,
			apic,
			io_apic_addr: None,

			flags,
		}
	}

	/// Returns the CPU's ID.
	pub fn get_id(&self) -> u32 {
		self.id
	}

	/// Returns a reference to the APIC.
	pub fn get_apic(&self) -> &APIC {
		&self.apic
	}

	/// Returns a mutable reference to the APIC.
	pub fn get_apic_mut(&mut self) -> &mut APIC {
		&mut self.apic
	}

	/// Tells whether the CPU is the current.
	pub fn is_current(&self) -> bool {
		self.apic.get_id() == get_current_id()
	}

	/// Returns the I/O APIC physical address.
	pub fn get_io_apic_addr(&self) -> Option<NonNull<u32>> {
		self.io_apic_addr
	}

	/// Sets the I/O APIC physical address.
	pub fn set_io_apic_addr(&mut self, addr: Option<NonNull<u32>>) {
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

	/// Enables another core with the given APIC ID `apic_id`. If already enabled, the behaviour is
	/// undefined.
	pub fn enable_other(&self, apic_id: u8) {
		let apic_id = apic_id as u32;

		self.apic.reg_write(apic::REG_OFFSET_ERROR_STATUS, 0);

		// TODO Use send_ipi
		let icr1 = (self.apic.reg_read(apic::REG_OFFSET_ICR1) & 0x00ffffff) | (apic_id << 24);
		self.apic.reg_write(apic::REG_OFFSET_ICR1, icr1);
		let icr0 = (self.apic.reg_read(apic::REG_OFFSET_ICR0) & 0xfff00000) | 0xc500;
		self.apic.reg_write(apic::REG_OFFSET_ICR0, icr0);
		self.apic.wait_delivery();

		// TODO Use send_ipi
		let icr1 = (self.apic.reg_read(apic::REG_OFFSET_ICR1) & 0x00ffffff) | (apic_id << 24);
		self.apic.reg_write(apic::REG_OFFSET_ICR1, icr1);
		let icr0 = (self.apic.reg_read(apic::REG_OFFSET_ICR0) & 0xfff00000) | 0x8500;
		self.apic.reg_write(apic::REG_OFFSET_ICR0, icr0);
		self.apic.wait_delivery();

		time::mdelay(10);

		for _ in 0..2 {
			self.apic.reg_write(apic::REG_OFFSET_ERROR_STATUS, 0);

			// TODO Use send_ipi
			let icr1 = (self.apic.reg_read(apic::REG_OFFSET_ICR1) & 0x00ffffff) | (apic_id << 24);
			self.apic.reg_write(apic::REG_OFFSET_ICR1, icr1);
			let icr0 = (self.apic.reg_read(apic::REG_OFFSET_ICR0) & 0xfff0f800) | 0x000608;
			self.apic.reg_write(apic::REG_OFFSET_ICR0, icr0);

			time::udelay(200);

			self.apic.wait_delivery();
		}
	}
}

/// The list of CPUs on the system.
static mut CPUS: Vec<IntMutex<CPU>> = Vec::new();

/// Returns the number of CPUs on the system.
pub fn get_count() -> usize {
	unsafe {
		CPUS.len()
	}
}

/// Returns the APIC ID of the current running CPU.
pub fn get_current_id() -> u8 {
	unsafe {
		get_current_apic() as _
	}
}

/// Returns the current core.
pub fn get_current() -> &'static IntMutex<CPU> {
	// TODO Optimize
	let iter = unsafe {
		CPUS.iter()
	};
	for cpu in iter {
		let cpu_guard = cpu.lock();
		let c = cpu_guard.get();

		if c.is_current() {
			drop(cpu_guard);
			return cpu;
		}
	}

	// TODO panic
	todo!();
}

/// Adds a new core to the core list.
/// This function is **not** thread-safe. Thus it is marked as unsafe.
pub unsafe fn add_core(cpu: CPU) -> Result<(), Errno> {
	CPUS.push(Mutex::new(cpu))
}

/// Returns a reference to the CPUs list's Mutex.
pub fn list() -> &'static Vec<IntMutex<CPU>> {
	unsafe {
		&CPUS
	}
}

/// Copies the trampoline code to its destination address to ensure it is accessible from real mode
/// CPUs.
/// The function also allocates stacks for each cores.
/// `cores_count` is the number of cores on the system.
/// The function returns the list of allocated stacks which will need to be freed later.
fn prepare_trampoline(cores_count: usize) -> Result<malloc::Alloc<*mut u8>, Errno> {
	// Allocating stacks
	let stacks = malloc::Alloc::<*mut u8>::new_fill(cores_count, | _ | {
		let stack_bottom = unsafe {
			malloc::alloc(CORE_STACK_SIZE)
		}? as *mut u8;
		let stack_top = unsafe {
			stack_bottom.add(CORE_STACK_SIZE)
		};

		Ok(stack_top)
	})?;

	// The location of the trampoline code in the kernel
	let src = unsafe {
		transmute::<unsafe extern "C" fn(), *const c_void>(cpu_trampoline)
	};
	// The location in the virtual memory at which the trampoline will be copied
	let dest = memory::kern_to_virt(TRAMPOLINE_PTR) as *mut c_void;
	// The size of the trampoline
	let size = unsafe {
		&trampoline_end as *const _ as usize
	} - src as usize;

	// The pointer to the kernel virtual memory handler
	let kern_vmem_ptr = unsafe {
		let vmem = crate::get_vmem().get_mut_payload().as_mut().unwrap();

		// Mapping the trampoline's code
		let pages = math::ceil_division(size, memory::PAGE_SIZE);
		vmem.map_range(TRAMPOLINE_PTR, TRAMPOLINE_PTR, pages, vmem::x86::FLAG_WRITE)?;

		let virt_ptr = (&*(vmem.as_ptr() as *const vmem::x86::X86VMem)).get_page_dir();
		memory::kern_to_phys(virt_ptr as _) as _
	};

	unsafe {
		vmem::write_lock_wrap(|| {
			// Copying trampoline code
			ptr::copy_nonoverlapping(src, dest, size);

			// Copying stacks array
			let stacks_offset = (&mut trampoline_stacks as *mut _ as usize) - (src as usize);
			let stacks_ptr = ((dest as usize) + stacks_offset) as *mut *const *mut u8;
			*stacks_ptr = stacks.as_ptr();

			// Copying kernel vmem pointer
			let vmem_offset = (&mut trampoline_vmem as *mut _ as usize) - (src as usize);
			let vmem_ptr = ((dest as usize) + vmem_offset) as *mut *mut u32;
			*vmem_ptr = kern_vmem_ptr;
		});
	}

	Ok(stacks)
}

// TODO Clean
/// Initializes CPU cores other than the main core.
/// If more than one CPU core is present on the system, the PIC is disabled and the APIC is enabled
/// instead.
/// This function must be called **only once, at boot**.
pub fn init_multicore() {
	let cores_count = get_count();

	// If there is not multiple cores, the function does nothing
	if cores_count <= 1 {
		return;
	}

	// Disabling legacy PIC
	// TODO Remove PIT timer
	pic::disable();

	// Preparing the trampoline to launch other cores
	let _stacks = prepare_trampoline(cores_count).unwrap_or_else(| _ | {
		crate::kernel_panic!("Failed to initialize multicore");
	});


	// Starting other CPUs
	let mut cpu_iter = unsafe { // Safe because running on single thread
		CPUS.iter()
	};
	for cpu in &mut cpu_iter {
		let mut cpu_guard = cpu.lock();
		let cpu = cpu_guard.get_mut();

		if cpu.is_current() {
			// Enabling main CPU's APIC
			cpu.get_apic_mut().enable();
		} else if cpu.can_enable() {
			// TODO Ensure the first is the main
			let main_cpu_guard = unsafe {
				CPUS[0].lock()
			};
			let main_cpu = main_cpu_guard.get();

			main_cpu.enable_other(cpu.get_apic().get_id());
		}
	}

	// Trampoline stacks are volontarily leaked to remain usable by their respective core when
	// needed
	// TODO Register stacks in a table to replace schedulers' tmp stacks?
}

/// The function to be called at the end of an interrupt.
#[no_mangle]
pub extern "C" fn end_of_interrupt(irq: u8) {
	let curr_cpu_guard = get_current().lock();
	let curr_cpu = curr_cpu_guard.get();
	let apic = curr_cpu.get_apic();

	if apic.is_enabled() {
		apic.end_of_interrupt(irq);
	} else {
		pic::end_of_interrupt(irq);
	}
}
