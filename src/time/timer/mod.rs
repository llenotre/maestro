//! This module implements timers.

pub mod divider;
pub mod pit;

/// A timer is an object which calls a given function at a given frequency.
pub trait Timer {
	/// Returns the name of the timer.
	fn get_name(&self) -> &str;

	/// Returns the maximum frequency of the timer in hertz.
	fn get_max_frequency(&self) -> u64;
	/// Returns the current frequency of the timer in hertz.
	fn get_curr_frequency(&self) -> u64;
	/// Sets the current frequency of the timer in hertz. The timer is approximating the given
	/// frequency to the closest supported.
	fn set_curr_frequency(&self) -> u64;
}

// TODO
