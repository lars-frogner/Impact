//! A fixed-capacity circular queue (ring buffer) implementation.

use allocator_api2::{
    alloc::{Allocator, Global},
    vec::Vec as AVec,
};

/// A fixed-capacity circular queue (ring buffer) that stores elements in FIFO
/// order.
///
/// The queue is initialized with a fixed capacity and cannot grow beyond that
/// size. Elements are stored in a circular fashion, wrapping around to the
/// beginning when the end is reached. This provides constant-time insertion
/// and removal operations.
///
/// # Note
/// The queue must be initialized with at least one element to establish its
/// capacity. Once created, the capacity cannot be changed.
#[derive(Clone, Debug)]
pub struct FixedQueue<T, A: Allocator = Global> {
    queue: AVec<T, A>,
    first: usize,
    last: usize,
    len: usize,
}

impl<T: Copy> FixedQueue<T> {
    /// Creates a new queue, initialized with the given values.
    ///
    /// The capacity of the queue is determined by the number of values
    /// provided. The queue will be full after creation.
    pub fn new_full(values: &[T]) -> Self {
        Self::new_full_in(Global, values)
    }
}

impl<T: Copy, A: Allocator> FixedQueue<T, A> {
    /// Creates a new queue with the specified allocator, initialized with the
    /// given values.
    ///
    /// The capacity of the queue is determined by the number of values
    /// provided. The queue will be full after creation.
    pub fn new_full_in(alloc: A, values: &[T]) -> Self {
        let mut queue = AVec::new_in(alloc);
        queue.extend_from_slice(values);
        let len = queue.len();
        let first = 0;
        let last = len.saturating_sub(1);
        Self {
            queue,
            first,
            last,
            len,
        }
    }

    /// Returns the maximum number of elements the queue can hold.
    ///
    /// The capacity is fixed at creation time and never changes.
    pub fn capacity(&self) -> usize {
        self.queue.len()
    }

    /// Whether the queue contains no elements.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of elements currently in the queue.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Removes and returns the first element from the queue, or returns `None`
    /// if the queue is empty.
    pub fn pop_front(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        let value = self.queue[self.first];

        self.first += 1;
        if self.first == self.capacity() {
            self.first = 0;
        }
        self.len -= 1;

        Some(value)
    }

    /// Adds an element to the back of the queue.
    ///
    /// # Panics
    /// If the queue is at full capacity.
    pub fn push_back(&mut self, value: T) {
        assert!(self.len() < self.capacity());

        self.last += 1;
        if self.last == self.capacity() {
            self.last = 0;
        }
        self.len += 1;

        self.queue[self.last] = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_queue(values: &[i32]) -> FixedQueue<i32> {
        FixedQueue::new_full(values)
    }

    #[test]
    fn creating_queue_with_values_gives_correct_state() {
        let queue = create_queue(&[10, 20, 30, 40]);

        assert_eq!(queue.capacity(), 4);
        assert_eq!(queue.len(), 4);
        assert!(!queue.is_empty());
    }

    #[test]
    fn creating_queue_with_single_value_works() {
        let queue = create_queue(&[42]);

        assert_eq!(queue.capacity(), 1);
        assert_eq!(queue.len(), 1);
        assert!(!queue.is_empty());
    }

    #[test]
    fn creating_queue_with_empty_values_works() {
        let queue = create_queue(&[]);
        assert_eq!(queue.capacity(), 0);
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn pop_front_from_full_queue_gives_first_value() {
        let mut queue = create_queue(&[10, 20, 30, 40]);

        let value = queue.pop_front();

        assert_eq!(value, Some(10));
        assert_eq!(queue.len(), 3);
        assert!(!queue.is_empty());
    }

    #[test]
    fn pop_front_from_single_item_queue_empties_queue() {
        let mut queue = create_queue(&[42]);

        let value = queue.pop_front();

        assert_eq!(value, Some(42));
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn pop_front_from_empty_queue_gives_none() {
        let mut queue = create_queue(&[42]);
        queue.pop_front(); // Make it empty

        let value = queue.pop_front();

        assert_eq!(value, None);
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn multiple_pop_front_operations_maintain_fifo_order() {
        let mut queue = create_queue(&[10, 20, 30, 40]);

        assert_eq!(queue.pop_front(), Some(10));
        assert_eq!(queue.pop_front(), Some(20));
        assert_eq!(queue.pop_front(), Some(30));
        assert_eq!(queue.pop_front(), Some(40));
        assert_eq!(queue.pop_front(), None);
    }

    #[test]
    fn push_back_to_partially_full_queue_adds_value() {
        let mut queue = create_queue(&[10, 20, 30]);
        queue.pop_front(); // Remove one to make space

        queue.push_back(50);

        assert_eq!(queue.len(), 3);
        assert!(!queue.is_empty());
    }

    #[test]
    #[should_panic]
    fn push_back_to_full_queue_fails() {
        let mut queue = create_queue(&[10, 20, 30, 40]);
        queue.push_back(50);
    }

    #[test]
    fn push_back_and_pop_front_maintain_fifo_behavior() {
        let mut queue = create_queue(&[10, 20]);
        queue.pop_front(); // Remove one item, leaving space
        queue.pop_front(); // Empty the queue

        queue.push_back(100);
        queue.push_back(200);

        assert_eq!(queue.pop_front(), Some(100));
        assert_eq!(queue.pop_front(), Some(200));
    }

    #[test]
    fn queue_operations_with_wrap_around_work_correctly() {
        let mut queue = create_queue(&[1, 2, 3]);

        // Pop two items and push two new ones to test wrap-around
        assert_eq!(queue.pop_front(), Some(1));
        assert_eq!(queue.pop_front(), Some(2));

        queue.push_back(4);
        queue.push_back(5);

        // Should maintain FIFO order
        assert_eq!(queue.pop_front(), Some(3));
        assert_eq!(queue.pop_front(), Some(4));
        assert_eq!(queue.pop_front(), Some(5));
        assert_eq!(queue.pop_front(), None);
    }

    #[test]
    fn alternating_push_pop_operations_work() {
        let mut queue = create_queue(&[10, 20]);

        assert_eq!(queue.pop_front(), Some(10));
        queue.push_back(30);
        assert_eq!(queue.pop_front(), Some(20));
        queue.push_back(40);
        assert_eq!(queue.pop_front(), Some(30));
        assert_eq!(queue.pop_front(), Some(40));
        assert!(queue.is_empty());
    }

    #[test]
    fn capacity_remains_constant() {
        let mut queue = create_queue(&[10, 20, 30, 40]);
        let initial_capacity = queue.capacity();

        queue.pop_front();
        queue.push_back(50);

        assert_eq!(queue.capacity(), initial_capacity);
    }

    #[test]
    fn len_updates_correctly_with_operations() {
        let mut queue = create_queue(&[1, 2, 3]);
        assert_eq!(queue.len(), 3);

        queue.pop_front();
        assert_eq!(queue.len(), 2);

        queue.pop_front();
        assert_eq!(queue.len(), 1);

        queue.push_back(4);
        assert_eq!(queue.len(), 2);

        queue.pop_front();
        queue.pop_front();
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn is_empty_reflects_queue_state() {
        let mut queue = create_queue(&[42]);
        assert!(!queue.is_empty());

        queue.pop_front();
        assert!(queue.is_empty());

        queue.push_back(99);
        assert!(!queue.is_empty());
    }
}
