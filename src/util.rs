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

    /// Whether an element exists at the given index. The index
    /// is allowed to be out of bounds.
    pub fn has_element_at_idx(&self, idx: usize) -> bool {
        idx < self.elements.len() && !self.free_list.contains(&idx)
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn creating_vec_works() {
        let vec = VecWithFreeList::<f32>::new();
        assert_eq!(vec.n_elements(), 0);
    }

    #[test]
    #[should_panic]
    fn demanding_element_from_empty_vec_fails() {
        let vec = VecWithFreeList::<f32>::new();
        vec.element(0);
    }

    #[test]
    #[should_panic]
    fn demanding_mutable_element_from_empty_vec_fails() {
        let mut vec = VecWithFreeList::<f32>::new();
        vec.element_mut(0);
    }

    #[test]
    fn requesting_element_from_empty_vec_gives_none() {
        let vec = VecWithFreeList::<f32>::new();
        assert!(vec.get_element(0).is_none());
    }

    #[test]
    fn requesting_mutable_element_from_empty_vec_gives_none() {
        let mut vec = VecWithFreeList::<f32>::new();
        assert!(vec.get_element_mut(0).is_none());
    }

    #[test]
    fn adding_to_empty_vec_works() {
        let mut vec = VecWithFreeList::<f32>::new();
        let idx = vec.add_element(1.0);

        assert_eq!(idx, 0);

        assert_eq!(vec.n_elements(), 1);

        assert_eq!(*vec.element(idx), 1.0);
        assert_eq!(*vec.element_mut(idx), 1.0);

        assert_eq!(vec.get_element(idx), Some(&1.0));
        assert_eq!(vec.get_element_mut(idx), Some(&mut 1.0));

        assert!(vec.has_element_at_idx(idx));
    }

    #[test]
    #[should_panic]
    fn freeing_out_of_bounds_idx_fails() {
        let mut vec = VecWithFreeList::<f32>::new();
        vec.free_element_at_idx(0);
    }

    #[test]
    fn freeing_only_element_in_vec_works() {
        let mut vec = VecWithFreeList::<f32>::new();
        let idx = vec.add_element(1.0);
        vec.free_element_at_idx(idx);

        assert_eq!(vec.n_elements(), 0);
        assert_eq!(vec.get_element(idx), None);
        assert_eq!(vec.get_element_mut(idx), None);
        assert!(!vec.has_element_at_idx(idx));
    }

    #[test]
    #[should_panic]
    fn demanding_freed_element_fails() {
        let mut vec = VecWithFreeList::<f32>::new();
        let idx = vec.add_element(1.0);
        vec.free_element_at_idx(idx);
        vec.element(idx);
    }

    #[test]
    #[should_panic]
    fn demanding_freed_mutable_element_fails() {
        let mut vec = VecWithFreeList::<f32>::new();
        let idx = vec.add_element(1.0);
        vec.free_element_at_idx(idx);
        vec.element_mut(idx);
    }

    #[test]
    fn requesting_freed_element_gives_none() {
        let mut vec = VecWithFreeList::<f32>::new();
        let idx = vec.add_element(1.0);
        vec.free_element_at_idx(idx);
        assert!(vec.get_element(idx).is_none());
    }

    #[test]
    fn requesting_freed_mutable_element_gives_none() {
        let mut vec = VecWithFreeList::<f32>::new();
        let idx = vec.add_element(1.0);
        vec.free_element_at_idx(idx);
        assert!(vec.get_element_mut(idx).is_none());
    }

    #[test]
    #[should_panic]
    fn freeing_freed_element_fails() {
        let mut vec = VecWithFreeList::<f32>::new();
        let idx = vec.add_element(1.0);
        vec.free_element_at_idx(idx);
        vec.free_element_at_idx(idx);
    }

    #[test]
    fn adding_after_free_uses_free_location() {
        let mut vec = VecWithFreeList::<f32>::new();
        let first_idx = vec.add_element(0.0);
        assert_eq!(first_idx, 0);
        assert_eq!(vec.n_elements(), 1);

        let second_idx = vec.add_element(1.0);
        assert_eq!(second_idx, 1);
        assert_eq!(vec.n_elements(), 2);

        vec.free_element_at_idx(first_idx);
        assert_eq!(vec.n_elements(), 1);

        let third_idx = vec.add_element(2.0);
        assert_eq!(third_idx, first_idx);
        assert_eq!(vec.n_elements(), 2);

        let fourth_idx = vec.add_element(3.0);
        assert_eq!(fourth_idx, 2);
        assert_eq!(vec.n_elements(), 3);
    }
}
