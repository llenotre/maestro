//! Functions to explore the kernel's ELF structures.

use super::{ELF32SectionHeader, ELF32Sym, SHT_SYMTAB};
use crate::{
	errno::{AllocResult, CollectResult},
	memory, multiboot, util,
	util::{container::hashmap::HashMap, lock::once::OnceInit},
};
use core::ffi::c_void;

/// A reference to the strtab.
static STRTAB: OnceInit<&'static ELF32SectionHeader> = unsafe { OnceInit::new() };
/// Name-to-symbol map for the kernel.
static SYMBOLS: OnceInit<HashMap<&'static [u8], ELF32Sym>> = unsafe { OnceInit::new() };

/// Returns an iterator over the kernel's ELF sections.
pub fn sections() -> impl Iterator<Item = &'static ELF32SectionHeader> {
	let boot_info = multiboot::get_boot_info();
	(0..boot_info.elf_num).map(|i| get_section_by_offset(i).unwrap())
}

/// Returns a reference to the `n`th kernel section.
///
/// If the section does not exist, the function returns `None`.
pub fn get_section_by_offset(n: u32) -> Option<&'static ELF32SectionHeader> {
	let boot_info = multiboot::get_boot_info();
	if n < boot_info.elf_num {
		let offset = n as usize * boot_info.elf_entsize as usize;
		let section = unsafe {
			let ptr = boot_info.elf_sections.add(offset);
			&*(memory::kern_to_virt(ptr) as *const ELF32SectionHeader)
		};
		Some(section)
	} else {
		None
	}
}

/// Returns the name of the given kernel ELF section.
///
/// If the name of the symbol could not be found, the function returns `None`.
pub fn get_section_name(section: &ELF32SectionHeader) -> Option<&'static [u8]> {
	let boot_info = multiboot::get_boot_info();
	// `unwrap` cannot fail because the ELF will always have this section
	let names_section = get_section_by_offset(boot_info.elf_shndx).unwrap();
	let ptr = memory::kern_to_virt((names_section.sh_addr + section.sh_name) as *const u8);
	// The string is in bound, otherwise the kernel's ELF is invalid
	Some(unsafe { util::str_from_ptr(ptr) })
}

/// Returns a reference to the kernel section with name `name`.
///
/// `name` is the name of the required section.
///
/// If the section doesn't exist, the function returns `None`.
pub fn get_section_by_name(name: &[u8]) -> Option<&'static ELF32SectionHeader> {
	sections().find(|s| get_section_name(s) == Some(name))
}

/// Returns an iterator over the kernel's ELF symbols.
pub fn symbols() -> impl Iterator<Item = &'static ELF32Sym> {
	sections()
		.filter(|section| section.sh_type == SHT_SYMTAB)
		.flat_map(|section| {
			let begin = memory::kern_to_virt(section.sh_addr as *const u8);
			let symbols_count = (section.sh_size / section.sh_entsize) as usize;
			(0..symbols_count).map(move |i| {
				let off = i * section.sh_entsize as usize;
				unsafe { &*(begin.add(off) as *const ELF32Sym) }
			})
		})
}

/// Returns the name of the given kernel ELF symbol.
///
/// If the name of the symbol could not be found, the function returns `None`.
pub fn get_symbol_name(symbol: &ELF32Sym) -> Option<&'static [u8]> {
	let ptr = memory::kern_to_virt((STRTAB.get().sh_addr + symbol.st_name) as *const u8);
	// The string is in bound, otherwise the kernel's ELF is invalid
	Some(unsafe { util::str_from_ptr(ptr) })
}

/// Returns the name of the kernel function for the given instruction pointer.
///
/// `inst` is the pointer to the instruction on the virtual memory.
///
/// If the name cannot be retrieved, the function returns `None`.
pub fn get_function_name(inst: *const c_void) -> Option<&'static [u8]> {
	symbols()
		.find(|sym| {
			let begin = sym.st_value as usize;
			let end = begin + sym.st_size as usize;
			(inst as usize) >= begin && (inst as usize) < end
		})
		.and_then(get_symbol_name)
}

/// Returns the kernel symbol with the name `name`.
///
/// `name` is the name of the symbol to get.
///
/// If the symbol doesn't exist, the function returns `None`.
pub fn get_symbol_by_name(name: &[u8]) -> Option<&'static ELF32Sym> {
	SYMBOLS.get().get(name)
}

/// Fills the kernel symbols map.
pub(crate) fn init() -> AllocResult<()> {
	// `.strtab` MUST be present
	// STRTAB must be initialized first because it is used to build the symbol map
	let strtab = get_section_by_name(b".strtab").unwrap();
	unsafe {
		STRTAB.init(strtab);
	}
	// Build the symbol map
	let map = symbols()
		.cloned()
		.filter_map(|sym| {
			let name = get_symbol_name(&sym)?;
			Some((name, sym))
		})
		.collect::<CollectResult<_>>()
		.0?;
	unsafe {
		SYMBOLS.init(map);
	}
	Ok(())
}