//! Buffers for the network stack.

/// A linked-list of buffers with no memory allocation, relying entirely on lifetimes.
pub struct BuffList<'buff> {
	/// The buffer.
	inner: &'buff [u8],

	/// The next buffer in the list.
	next: Option<&'buff BuffList<'buff>>,
	/// The length of following buffers combined.
	next_len: usize,
}

impl<'buff> From<&'buff [u8]> for BuffList<'buff> {
	fn from(inner: &'buff [u8]) -> Self {
		Self {
			inner,

			next: None,
			next_len: 0,
		}
	}
}

impl<'buff> BuffList<'buff> {
	/// Returns a reference to the inner buffer.
	pub fn inner(&self) -> &[u8] {
		self.inner
	}

	/// Returns the length of the buffer, plus following buffers.
	pub fn len(&self) -> usize {
		self.inner.len() + self.next_len
	}

	/// Pushes another buffer at the front of the current list.
	///
	/// The function returns the new head of the list (which is the given `front`).
	pub fn push_front<'other, 's: 'other>(
		&'s mut self,
		mut front: BuffList<'other>,
	) -> BuffList<'other>
	where
		'buff: 'other,
	{
		front.next = Some(self);
		front.next_len = self.inner.len() + self.next_len;

		front
	}

	/// Returns a iterator to the buffers list.
	pub fn iter(&self) -> BuffIter {
		BuffIter {
			buff: Some(self),
		}
	}
}

/// Iterator on `BuffList`.
pub struct BuffIter<'buff> {
	/// The buffers list.
	buff: Option<&'buff BuffList<'buff>>,
}

impl<'buff> Iterator for BuffIter<'buff> {
	type Item = &'buff [u8];

	fn next(&mut self) -> Option<Self::Item> {
		let b = self.buff?;
		self.buff = b.next;
		Some(b.inner)
	}
}
