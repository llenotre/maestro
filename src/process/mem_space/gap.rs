/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! A gap is a region of the virtual memory which is available for allocation.

use crate::memory;
use core::cmp::min;
use core::ffi::c_void;
use core::fmt;
use core::num::NonZeroUsize;

/// A gap in the memory space that can use for new mappings.
#[derive(Clone)]
pub struct MemGap {
	/// Pointer on the virtual memory to the beginning of the gap
	begin: *mut c_void,
	/// The size of the gap in pages.
	size: NonZeroUsize,
}

impl MemGap {
	/// Creates a new instance.
	///
	/// Arguments:
	/// - `begin` is a pointer on the virtual memory to the beginning of the gap.
	/// This pointer must be page-aligned.
	/// - `size` is the size of the gap in pages.
	pub fn new(begin: *mut c_void, size: NonZeroUsize) -> Self {
		debug_assert!(begin.is_aligned_to(memory::PAGE_SIZE));

		Self {
			begin,
			size,
		}
	}

	/// Returns a pointer on the virtual memory to the beginning of the gap.
	pub fn get_begin(&self) -> *mut c_void {
		self.begin
	}

	/// Returns a pointer on the virtual memory to the end of the gap.
	pub fn get_end(&self) -> *mut c_void {
		unsafe { self.begin.add(self.size.get() * memory::PAGE_SIZE) }
	}

	/// Returns the size of the gap in memory pages.
	pub fn get_size(&self) -> NonZeroUsize {
		self.size
	}

	/// Creates new gaps to replace the current one after mapping memory onto
	/// it.
	///
	/// After calling this function, the callee shall removed the current
	/// gap from its container and insert the new ones in it.
	///
	/// Arguments:
	/// - `off` is the offset of the part to consume.
	/// - `size` is the size of the part to consume.
	///
	/// The function returns a new gap. If the gap is fully consumed, the
	/// function returns `(None, None)`.
	pub fn consume(&self, off: usize, size: usize) -> (Option<Self>, Option<Self>) {
		// The new gap located before the mapping
		let left = NonZeroUsize::new(off).map(|off| {
			let addr = self.begin;
			let size = min(off, self.size);

			Self::new(addr, size)
		});

		// The new gap located after the mapping
		let right = self
			.size
			.get()
			.checked_sub(off + size)
			.and_then(NonZeroUsize::new)
			.map(|gap_size| {
				let addr = unsafe { self.begin.add((off + size) * memory::PAGE_SIZE) };

				Self::new(addr, gap_size)
			});

		(left, right)
	}

	/// Merges the given gap `other` with the current gap.
	///
	/// If the gaps are not adjacent, the function does nothing.
	pub fn merge(&mut self, other: Self) {
		if self.get_begin() == other.get_end() {
			// If `other` is before
			self.begin = other.begin;

			unsafe {
				self.size = self.size.unchecked_add(other.size.get());
			}
		} else if self.get_end() == other.get_begin() {
			// If `other` is after
			unsafe {
				self.size = self.size.unchecked_add(other.size.get());
			}
		}
	}
}

impl fmt::Debug for MemGap {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"MemGap {{ begin: {:p}, end: {:p} }}",
			self.begin,
			self.get_end()
		)
	}
}
