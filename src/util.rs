//! Generic utilities.

use std::collections::LinkedList;

/// A [`Vec`] that maintains a list of each index
/// where the element has been deleted and reuses
/// these locations when adding new items.
#[derive(Clone, Debug, Default)]
pub struct VecWithFreeList<T> {
    elements: Vec<T>,
    free_list: LinkedList<usize>,
}

impl<T> VecWithFreeList<T> {
    /// Creates a new empty vector.
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            free_list: LinkedList::new(),
        }
    }

    /// Creates a new empty vector with the given capacity
    /// pre-allocated.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            elements: Vec::with_capacity(capacity),
            free_list: LinkedList::new(),
        }
    }

    /// Returns the logical number of elements in the vector.
    /// This number does not include elements that have been
    /// deleted.
    pub fn n_elements(&self) -> usize {
        self.elements.len() - self.free_list.len()
    }

    /// Returns a reference to the element at the given index.
    ///
    /// # Panics
    /// If the index is out of bounds or refers to a location
    /// that is currently freed.
    pub fn element(&self, idx: usize) -> &T {
        assert!(
            !self.free_list.contains(&idx),
            "Tried to access element at vacant index"
        );
        &self.elements[idx]
    }

    /// Returns a mutable reference to the element at the given
    /// index.
    ///
    /// # Panics
    /// If the index is out of bounds or refers to a location
    /// that is currently freed.
    pub fn element_mut(&mut self, idx: usize) -> &mut T {
        assert!(
            !self.free_list.contains(&idx),
            "Tried to access element at vacant index"
        );
        &mut self.elements[idx]
    }

    /// Returns a reference to the element at the given index,
    /// or [`None`] if the index refers to a location that
    /// is currently freed.
    ///
    /// # Panics
    /// If the index is out of bounds.
    pub fn get_element(&self, idx: usize) -> Option<&T> {
        if idx >= self.elements.len() || self.free_list.contains(&idx) {
            None
        } else {
            Some(&self.elements[idx])
        }
    }

    /// Returns a mutable reference to the element at the given
    /// index, or [`None`] if the index refers to a location that
    /// is currently freed.
    ///
    /// # Panics
    /// If the index is out of bounds.
    pub fn get_element_mut(&mut self, idx: usize) -> Option<&mut T> {
        if idx >= self.elements.len() || self.free_list.contains(&idx) {
            None
        } else {
            Some(&mut self.elements[idx])
        }
    }

    /// Inserts the given element into the vector. If a freed
    /// location is available, this is used, otherwise the vector
    /// is grown in length and the element inserted at the end.
    ///
    /// # Returns
    /// The index where the element was added.
    pub fn add_element(&mut self, element: T) -> usize {
        match self.free_list.pop_front() {
            Some(free_idx) => {
                self.elements[free_idx] = element;
                free_idx
            }
            None => {
                let idx = self.elements.len();
                self.elements.push(element);
                idx
            }
        }
    }

    /// Removes the element at the given index. The underlying
    /// [`Vec`] is not modified, instead the index is registered
    /// as free.
    ///
    /// # Panics
    /// If the index is out of bounds or refers to a location
    /// that is currently freed.
    pub fn free_element_at_idx(&mut self, idx: usize) {
        assert!(
            idx < self.elements.len(),
            "Tried to free element past end of `Vec`"
        );
        assert!(
            !self.free_list.contains(&idx),
            "Tried to free element at vacant index"
        );
        self.free_list.push_back(idx);
    }
}
