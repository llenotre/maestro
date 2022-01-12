//! This module implements Direct Access Memory (DMA) features.
//!
//! DMA allows to access components of the system by reading or writing to the memory at specific
//! locations.

use core::ffi::c_void;
use crate::errno::Errno;
use crate::memory::vmem::VMem;
use crate::memory::vmem;
use crate::util::boxed::Box;
use crate::util::container::vec::Vec;
use crate::util::lock::Mutex;

/// Structure representing a DMA zone.
pub struct DMA {
    /// Pointer to the beginning of the DMA zone.
    phys_begin: *mut c_void,
    /// The size of the DMA zone in bytes.
    size: usize,

    /// The virtual pointer at which the zone is to be mapped.
    virt_ptr: *mut c_void,
}

impl DMA {
    /// Creates a new instance.
    pub fn new(phys_begin: *mut c_void, size: usize, virt_ptr: *mut c_void) -> Self {
        Self {
            phys_begin,
            size,

            virt_ptr,
        }
    }

    /// Maps the DMA zone onto the given virtual memory context handler.
    /// The function doesn't flush the modifications. It's the caller's responsibility to do so.
    pub fn map(&self, vmem: &mut Box<dyn VMem>) -> Result<(), Errno> {
        let dma_flags = vmem::x86::FLAG_CACHE_DISABLE | vmem::x86::FLAG_WRITE_THROUGH
			| vmem::x86::FLAG_WRITE;
        vmem.map_range(self.phys_begin, self.virt_ptr, self.size, dma_flags)
    }
}

/// The vector containing the list of registered DMA zones.
static mut ZONES: Mutex<Vec<DMA>> = Mutex::new(Vec::new());

/// Registers a new DMA zone.
pub fn register(dma: DMA) -> Result<(), Errno> {
    let mut guard = unsafe { // Safe because using Mutex
        ZONES.lock()
    };

    guard.get_mut().push(dma)
}

/// Maps the ACPI DMA zones on the given virtual memory context handler. If ACPI hasn't been
/// initialized yet, the function does nothing.
/// The function flushes the modifications.
pub fn map(vmem: &mut Box<dyn VMem>) -> Result<(), Errno> {
    let guard = unsafe { // Safe because using Mutex
        ZONES.lock()
    };

    // TODO Save a copy of `vmem` to restore if a mapping fails?
    for z in guard.get().iter() {
        z.map(vmem)?;
    }
    vmem.flush();

    Ok(())
}
