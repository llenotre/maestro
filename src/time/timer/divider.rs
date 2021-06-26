//! The frequency divider allows to take as input the signal of a timer and to divide it to target a lower frequency.

/// Structure representing the frequency divider.
pub struct FrequencyDivider<O: Fn()> {
	/// The number of signals to count before sending one signal to the output.
	count: u64,
	/// The number of signals counted so far.
	i: u64,

	/// The output callback.
	output: O,
}

impl<O: Fn()> FrequencyDivider<O> {
	/// Creates a new instance.
	pub fn new(count: u64, output: O) -> Self {
		Self {
			count,
			i: 0,

			output,
		}
	}

	/// Returns `b` in the formula `a / b = c`, where `a` is the input frequency and `c` is the output frequency.
	pub fn get_count(&self) -> u64 {
		self.count
	}

	/// Function to call to receive an input signal.
	pub fn input(&mut self) {
		self.i += 1;

		if self.i >= self.count {
			(self.output)();
			self.i = 0;
		}
	}
}
