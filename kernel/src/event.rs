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

//! This interface allows to register callbacks for each interrupts.

use crate::{
	crypto::{rand, rand::EntropyPool},
	idt,
	idt::pic,
	process::{regs::Regs, tss::TSS},
};
use core::{ffi::c_void, intrinsics::unlikely, ptr::NonNull};
use utils::{boxed::Box, collections::vec::Vec, errno::AllocResult, lock::IntMutex};

/// The list of interrupt error messages ordered by index of the corresponding
/// interrupt vector.
#[cfg(target_arch = "x86")]
static ERROR_MESSAGES: &[&str] = &[
	"Divide-by-zero Error",
	"Debug",
	"Non-maskable Interrupt",
	"Breakpoint",
	"Overflow",
	"Bound Range Exceeded",
	"Invalid Opcode",
	"Device Not Available",
	"Double Fault",
	"Coprocessor Segment Overrun",
	"Invalid TSS",
	"Segment Not Present",
	"Stack-Segment Fault",
	"General Protection Fault",
	"Page Fault",
	"Unknown",
	"x87 Floating-Point Exception",
	"Alignement Check",
	"Machine Check",
	"SIMD Floating-Point Exception",
	"Virtualization Exception",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Security Exception",
	"Unknown",
];

/// Returns the error message corresponding to the given interrupt vector index
/// `i`.
fn get_error_message(i: u32) -> &'static str {
	if (i as usize) < ERROR_MESSAGES.len() {
		ERROR_MESSAGES[i as usize]
	} else {
		"Unknown"
	}
}

/// The action to execute after the interrupt handler has returned.
pub enum CallbackResult {
	/// Executes remaining callbacks for the interrupt.
	///
	/// If this is the last callback to be executed, the execution resumes the code that was
	/// interrupted.
	Continue,
	/// Makes the current CPU kernel idle until the next interruption.
	Idle,
	/// Makes the kernel panic with a message corresponding to the interruption.
	Panic,
}

/// A callback to handle an interruption.
///
/// Arguments:
/// - `id` is the id of the interrupt.
/// - `code` is an optional code associated with the interrupt. If no code is given, the value
/// is `0`.
/// - `regs` the values of the registers when the interruption was triggered.
/// - `ring` tells the ring at which the code was running.
///
/// The return value tells which action to perform next.
type CallbackWrapper = Box<dyn FnMut(u32, u32, &Regs, u32) -> CallbackResult>;

/// Structure used to detect whenever the object owning the callback is
/// destroyed, allowing to unregister it automatically.
#[must_use]
pub struct CallbackHook {
	/// The id of the interrupt the callback is bound to.
	id: u32,
	/// The pointer of the callback.
	ptr: NonNull<c_void>,
}

impl Drop for CallbackHook {
	fn drop(&mut self) {
		// Remove the callback
		let mut vec = CALLBACKS[self.id as usize].lock();
		let i = vec
			.iter()
			.enumerate()
			.find(|(_, c)| c.as_ptr() as *mut c_void == self.ptr.as_ptr())
			.map(|(i, _)| i);
		if let Some(i) = i {
			vec.remove(i);
		}
	}
}

/// The default value for `CALLBACKS`.
#[allow(clippy::declare_interior_mutable_const)]
const CALLBACKS_INIT: IntMutex<Vec<CallbackWrapper>> = IntMutex::new(Vec::new());
/// List containing vectors that store callbacks for every interrupt watchdogs.
static CALLBACKS: [IntMutex<Vec<CallbackWrapper>>; idt::ENTRIES_COUNT as _] =
	[CALLBACKS_INIT; idt::ENTRIES_COUNT as _];

/// Registers the given callback and returns a reference to it.
///
/// The latest registered callback is executed last. Thus, callback that are registered before can
/// prevent next callbacks from being executed.
///
/// Arguments:
/// - `id` is the id of the interrupt to watch.
/// - `callback` is the callback to register.
///
/// If an allocation fails, the function shall return an error.
///
/// If the provided ID is invalid, the function returns `None`.
pub fn register_callback<C>(id: u32, callback: C) -> AllocResult<Option<CallbackHook>>
where
	C: 'static + FnMut(u32, u32, &Regs, u32) -> CallbackResult,
{
	if unlikely(id as usize >= CALLBACKS.len()) {
		return Ok(None);
	}

	let mut vec = CALLBACKS[id as usize].lock();
	let b = Box::new(callback)?;
	let ptr = b.as_ptr();
	vec.push(b)?;

	Ok(Some(CallbackHook {
		id,
		ptr: NonNull::new(ptr as _).unwrap(),
	}))
}

/// Unlocks the callback vector with id `id`. This function is to be used in
/// case of an event callback that never returns.
///
/// # Safety
///
/// This function is marked as unsafe since it may lead to concurrency issues if
/// not used properly. It must be called from the same CPU kernel as the one that
/// locked the mutex since unlocking changes the interrupt flag.
#[no_mangle]
pub unsafe extern "C" fn unlock_callbacks(id: usize) {
	CALLBACKS[id].unlock();
}

/// Feeds the entropy pool using the given data.
fn feed_entropy<T>(pool: &mut EntropyPool, val: &T) {
	let buff = utils::bytes::as_bytes(val);
	pool.write(buff);
}

/// This function is called whenever an interruption is triggered.
///
/// Arguments:
/// - `id` is the identifier of the interrupt type
/// This value is architecture-dependent
/// - `code` is an optional code associated with the interrupt
/// If the interrupt type doesn't have a code, the value is `0`
/// - `regs` is the state of the registers at the moment of the interrupt
/// - `ring` tells the ring at which the code was running
#[no_mangle]
extern "C" fn event_handler(id: u32, code: u32, ring: u32, regs: &Regs) {
	// Feed entropy pool
	{
		let mut pool = rand::ENTROPY_POOL.lock();
		if let Some(pool) = &mut *pool {
			feed_entropy(pool, &id);
			feed_entropy(pool, &code);
			feed_entropy(pool, &ring);
			feed_entropy(pool, regs);
		}
	}

	let mut callbacks = CALLBACKS[id as usize].lock();
	for c in callbacks.iter_mut() {
		let result = c(id, code, regs, ring);
		match result {
			CallbackResult::Continue => {}

			CallbackResult::Idle => {
				// Unlock to avoid deadlocks
				if id >= ERROR_MESSAGES.len() as u32 {
					pic::end_of_interrupt((id - ERROR_MESSAGES.len() as u32) as _);
				}
				drop(callbacks);

				unsafe {
					crate::loop_reset(TSS.0.esp0 as _);
				}
			}

			CallbackResult::Panic => panic!("{}, code: {code:x}", get_error_message(id)),
		}
	}
}
