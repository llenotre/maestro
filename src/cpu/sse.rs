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

//! SSE-related features.

/// Tells whether the CPU supports SSE.
pub fn is_present() -> bool {
	unsafe { super::cpuid_has_sse() }
}

/// Enables SSE.
pub fn enable() {
	unsafe {
		super::cr0_clear(0b100); // Enable x87 FPU
		super::cr0_set(0b10);

		// Enable FXSAVE and FXRSTOR (thus, enabling SSE) and SSE exceptions
		let cr4 = super::cr4_get() | 0b11000000000;
		super::cr4_set(cr4);
	}
}
