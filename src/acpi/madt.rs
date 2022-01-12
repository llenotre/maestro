//! This modules handles ACPI's Multiple APIC Description Table (MADT).

use super::ACPITable;
use super::ACPITableHeader;

/// The offset of the entries in the MADT.
const ENTRIES_OFF: usize = 0x2c;

/// Entry describing a processor local APIC.
pub const ENTRY_PROCESSOR_LOCAL_APIC: u8 = 0;
/// Entry describing an I/O APIC.
pub const ENTRY_IO_APIC: u8 = 1;
/// Entry describing a local APIC address override.
pub const ENTRY_LOCAL_APIC_ADDRESS_OVERRIDE: u8 = 5;
// TODO Add all types

/// The Multiple APIC Description Table.
#[repr(C)]
pub struct Madt {
	/// The table's header.
	pub header: ACPITableHeader,

	/// The physical address at which each process can access its local interrupt controller.
	pub local_apic_addr: u32,
	/// APIC flags.
	pub flags: u32,
}

impl Madt {
	/// Executes the given closure for each entry in the MADT.
	pub fn foreach_entry<F: Fn(&EntryHeader)>(&self, f: F) {
		let entries_len = self.header.get_length() as usize - ENTRIES_OFF;

		let mut i = 0;
		while i < entries_len {
			let entry = unsafe {
				&*((self as *const _ as usize + ENTRIES_OFF + i) as *const EntryHeader)
			};

			f(entry);

			i += entry.get_length() as usize;
		}
	}
}

impl ACPITable for Madt {
	fn get_expected_signature() -> [u8; 4] {
		[b'A', b'P', b'I', b'C']
	}
}

/// Represents an MADT entry header.
#[repr(C)]
pub struct EntryHeader {
	/// The entry type.
	entry_type: u8,
	/// The entry length.
	length: u8,
}

impl EntryHeader {
	/// Returns the type of the entry.
	pub fn get_type(&self) -> u8 {
		self.entry_type
	}

	/// Returns the length of the entry.
	pub fn get_length(&self) -> u8 {
		self.length
	}
}

/// Processor Local APIC entry structure.
#[repr(C)]
pub struct EntryProcessorLocalAPIC {
	/// The entry header.
	pub header: EntryHeader,

	/// The processor ID.
	pub id: u8,
	/// The APIC ID.
	pub apic_id: u8,
	/// Flags.
	pub flags: u32,
}

/// I/O APIC entry structure.
#[repr(C)]
pub struct EntryIOAPIC {
	/// The entry header.
	pub header: EntryHeader,

	/// The I/O APIC ID.
	pub io_apic_id: u8,
	/// Reserved byte.
	reserved: u8,
	/// The I/O APIC address.
	pub io_apic_addr: u32,
	/// The global system interrupt base.
	pub global_system_interrupt_base: u32,
}

/// Local APIC Address Override entry structure.
#[repr(C)]
pub struct EntryLocalAPICAddressOverride {
	/// The entry header.
	pub header: EntryHeader,

	/// Reserved word.
	reserved: u16,
	/// The local APIC physical address.
	pub local_apic_addr: u64,
}
