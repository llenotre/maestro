//! Buffers for the network stack.

use core::ptr::NonNull;

/// A linked-list of buffers with no memory allocation, relying entirely on lifetimes.
pub struct BuffList<'buff> {
	/// The buffer.
	b: &'buff [u8],

	/// The next buffer in the list.
	next: Option<NonNull<BuffList<'buff>>>,
	/// The length of following buffers combined.
	next_len: usize,
}

impl<'buff> From<&'buff [u8]> for BuffList<'buff> {
	fn from(b: &'buff [u8]) -> Self {
		Self {
			b,

			next: None,
			next_len: 0,
		}
	}
}

impl<'buff> BuffList<'buff> {
	/// Returns the length of the buffer, plus following buffers.
	pub fn len(&self) -> usize {
		self.b.len() + self.next_len
	}

	/// Pushes another buffer at the front of the current list.
	///
	/// The function returns the new head of the list (which is the given `front`).
	pub fn push_front<'other>(&mut self, mut front: BuffList<'other>) -> BuffList<'other>
	where
		'buff: 'other,
	{
		front.next = NonNull::new(self);
		front.next_len = self.b.len() + self.next_len;

		front
	}
}
