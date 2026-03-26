/// A generic fixed-capacity circular buffer optimised for time-series data.
///
/// When the buffer is full, `push` silently overwrites the oldest entry so
/// that the most-recent `capacity` items are always available with O(1)
/// writes and O(n) iteration.
pub struct RingBuffer<T> {
    data: Vec<Option<T>>,
    /// Index where the *next* item will be written.
    head: usize,
    /// Number of items currently stored (≤ capacity).
    len: usize,
}

impl<T: Clone> RingBuffer<T> {
    /// Create a new buffer with the given fixed capacity.
    ///
    /// # Panics
    /// Panics if `capacity` is zero.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "RingBuffer capacity must be greater than zero");
        Self {
            data: vec![None; capacity],
            head: 0,
            len: 0,
        }
    }

    /// The maximum number of items the buffer can hold.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.data.len()
    }

    /// The number of items currently stored.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` when no items are stored.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns `true` when the buffer holds exactly `capacity` items.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.len == self.capacity()
    }

    /// Push an item onto the buffer.
    ///
    /// If the buffer is already full, the oldest item is silently overwritten.
    /// This operation is O(1).
    pub fn push(&mut self, item: T) {
        self.data[self.head] = Some(item);
        self.head = (self.head + 1) % self.capacity();
        if self.len < self.capacity() {
            self.len += 1;
        }
    }

    /// Return a reference to the most recently pushed item, or `None` if the
    /// buffer is empty.
    pub fn latest(&self) -> Option<&T> {
        if self.is_empty() {
            return None;
        }
        // `head` points at the *next* write slot, so the latest item is one
        // position behind it (wrapping).
        let idx = (self.head + self.capacity() - 1) % self.capacity();
        self.data[idx].as_ref()
    }

    /// Return an iterator that yields items from **oldest to newest**.
    pub fn iter(&self) -> RingBufferIter<'_, T> {
        RingBufferIter::new(self, false)
    }

    /// Return an iterator that yields items from **newest to oldest**.
    pub fn iter_rev(&self) -> RingBufferIter<'_, T> {
        RingBufferIter::new(self, true)
    }

    /// Remove all items from the buffer without changing its capacity.
    pub fn clear(&mut self) {
        for slot in &mut self.data {
            *slot = None;
        }
        self.head = 0;
        self.len = 0;
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Return the physical index of the *oldest* item.
    ///
    /// When the buffer is not full, the oldest item is always at index 0
    /// (items are written sequentially from 0 until the buffer wraps).
    /// Once the buffer is full, the oldest item is at `head` because `head`
    /// is about to be overwritten next.
    #[inline]
    fn oldest_index(&self) -> usize {
        if self.is_full() {
            self.head
        } else {
            0
        }
    }
}

// ---------------------------------------------------------------------------
// Iterator
// ---------------------------------------------------------------------------

/// An iterator over a [`RingBuffer`].
///
/// Direction is chosen at construction time:
/// - `reversed = false` => oldest-to-newest
/// - `reversed = true`  => newest-to-oldest
pub struct RingBufferIter<'a, T> {
    buffer: &'a RingBuffer<T>,
    /// How many items have been yielded so far.
    yielded: usize,
    reversed: bool,
}

impl<'a, T: Clone> RingBufferIter<'a, T> {
    fn new(buffer: &'a RingBuffer<T>, reversed: bool) -> Self {
        Self {
            buffer,
            yielded: 0,
            reversed,
        }
    }
}

impl<'a, T: Clone> Iterator for RingBufferIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.yielded >= self.buffer.len() {
            return None;
        }

        let cap = self.buffer.capacity();
        let oldest = self.buffer.oldest_index();

        let physical_idx = if self.reversed {
            // newest first: start from (oldest + len - 1) and count back
            let newest_offset = self.buffer.len() - 1 - self.yielded;
            (oldest + newest_offset) % cap
        } else {
            // oldest first
            (oldest + self.yielded) % cap
        };

        self.yielded += 1;
        self.buffer.data[physical_idx].as_ref()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.buffer.len() - self.yielded;
        (remaining, Some(remaining))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer_is_empty() {
        let buf: RingBuffer<i32> = RingBuffer::new(4);
        assert_eq!(buf.len(), 0);
        assert!(buf.is_empty());
        assert!(!buf.is_full());
        assert!(buf.latest().is_none());
    }

    #[test]
    fn test_push_and_latest() {
        let mut buf = RingBuffer::new(4);
        buf.push(1);
        assert_eq!(buf.latest(), Some(&1));
        buf.push(2);
        assert_eq!(buf.latest(), Some(&2));
        buf.push(99);
        assert_eq!(buf.latest(), Some(&99));
        assert_eq!(buf.len(), 3);
        assert!(!buf.is_full());
    }

    #[test]
    fn test_overwrites_when_full() {
        let mut buf = RingBuffer::new(3);
        buf.push(1);
        buf.push(2);
        buf.push(3); // buffer is now full: [1, 2, 3]
        assert!(buf.is_full());

        buf.push(4); // overwrites 1 => [2, 3, 4]
        assert_eq!(buf.len(), 3);
        assert_eq!(buf.latest(), Some(&4));

        let items: Vec<_> = buf.iter().copied().collect();
        assert_eq!(items, vec![2, 3, 4]);
    }

    #[test]
    fn test_iter_ordering() {
        let mut buf = RingBuffer::new(5);
        for i in 1..=5 {
            buf.push(i);
        }
        let fwd: Vec<_> = buf.iter().copied().collect();
        assert_eq!(fwd, vec![1, 2, 3, 4, 5]);

        let rev: Vec<_> = buf.iter_rev().copied().collect();
        assert_eq!(rev, vec![5, 4, 3, 2, 1]);
    }

    #[test]
    fn test_iter_after_wrap() {
        let mut buf = RingBuffer::new(4);
        // Fill completely then push two more to wrap around twice
        for i in 1..=6 {
            buf.push(i);
        }
        // Buffer should hold the last 4: [3, 4, 5, 6]
        assert_eq!(buf.len(), 4);
        assert!(buf.is_full());

        let fwd: Vec<_> = buf.iter().copied().collect();
        assert_eq!(fwd, vec![3, 4, 5, 6]);

        let rev: Vec<_> = buf.iter_rev().copied().collect();
        assert_eq!(rev, vec![6, 5, 4, 3]);
    }

    #[test]
    fn test_clear() {
        let mut buf = RingBuffer::new(3);
        buf.push(10);
        buf.push(20);
        buf.push(30);
        assert!(buf.is_full());

        buf.clear();
        assert_eq!(buf.len(), 0);
        assert!(buf.is_empty());
        assert!(!buf.is_full());
        assert!(buf.latest().is_none());
        assert_eq!(buf.iter().count(), 0);

        // Ensure the buffer is still usable after clearing
        buf.push(42);
        assert_eq!(buf.latest(), Some(&42));
        assert_eq!(buf.len(), 1);
    }

    #[test]
    #[should_panic]
    fn test_zero_capacity_panics() {
        let _buf: RingBuffer<i32> = RingBuffer::new(0);
    }

    #[test]
    fn test_single_capacity() {
        let mut buf = RingBuffer::new(1);
        assert!(buf.is_empty());

        buf.push(1);
        assert_eq!(buf.len(), 1);
        assert!(buf.is_full());
        assert_eq!(buf.latest(), Some(&1));

        buf.push(2);
        assert_eq!(buf.len(), 1);
        assert!(buf.is_full());
        assert_eq!(buf.latest(), Some(&2));

        let items: Vec<_> = buf.iter().copied().collect();
        assert_eq!(items, vec![2]);

        let rev: Vec<_> = buf.iter_rev().copied().collect();
        assert_eq!(rev, vec![2]);
    }
}
