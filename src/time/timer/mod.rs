//! This module implements timers.

use crate::util::math::rational::Rational;

pub mod divider;
pub mod pit;

/// A timer is an object which calls a given function at a given frequency.
pub trait Timer {
	/// Returns the name of the timer.
	fn get_name(&self) -> &str;

	/// Returns the maximum frequency of the timer in hertz.
	fn get_max_frequency(&self) -> Rational;
	/// Returns the current frequency of the timer in hertz.
	fn get_curr_frequency(&self) -> Rational;
	/// Sets the current frequency of the timer in hertz. The timer is approximating the given
	/// frequency to the closest supported. To get the exact frequency, one should use
	/// `get_curr_frequency` after setting it.
	/// If the given frequency is negative, the behaviour is undefined.
	fn set_curr_frequency(&self, frequency: Rational);
}
