//! A [`Vec`] that maintains a list of each index where
//! the element has been deleted and reuses these locations
//! when adding new items.

use bytemuck::{Pod, Zeroable};
use roc_codegen::roc;
use std::{cmp, collections::VecDeque};

/// A [`Vec`] that maintains a list of each index where
/// the element has been deleted and reuses these locations
/// when adding new items.
///
/// In order to prevent use-after-free issues, each location
/// has an associated "generation" that is advanced every time
/// the location is reused after a free. The generation is
/// contained in the returned [`GenerationalIdx`]. Every time
/// an element is to be accessed, the generation of the index
/// is compared with the current generation of the location
/// being indexed, and the access is rejected if the generations
/// do not match.
#[derive(Clone, Debug, Default)]
pub struct GenerationalReusingVec<T> {
    elements: Vec<GenerationalElement<T>>,
    free_list: VecDeque<usize>,
}

/// An index into a [`GenerationalReusingVec`].
#[roc(parents = "Containers")]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
pub struct GenerationalIdx {
    generation: Generation,
    idx: usize,
}

#[derive(Clone, Debug)]
struct GenerationalElement<T> {
    generation: Generation,
    element: T,
}

type Generation = usize;

impl<T> GenerationalReusingVec<T> {
    /// Creates a new empty vector.
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            free_list: VecDeque::new(),
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
    /// If the index:
    /// - Refers to a location that is currently freed.
    /// - Does not have the same generation as the element.
    /// - Is out of bounds (in which case it belongs to a
    ///   different vector).
    pub fn element(&self, gen_idx: GenerationalIdx) -> &T {
        assert!(
            !self.free_list.contains(&gen_idx.idx()),
            "Tried to access element at freed index"
        );
        self.elements[gen_idx.idx()]
            .get_element(gen_idx.generation())
            .expect("Tried to access element with outdated generation")
    }

    /// Returns a mutable reference to the element at the given
    /// index.
    ///
    /// # Panics
    /// If the index:
    /// - Refers to a location that is currently freed.
    /// - Does not have the same generation as the element.
    /// - Is out of bounds (in which case it belongs to a
    ///   different vector).
    pub fn element_mut(&mut self, gen_idx: GenerationalIdx) -> &mut T {
        assert!(
            !self.free_list.contains(&gen_idx.idx()),
            "Tried to access element at freed index"
        );
        self.elements[gen_idx.idx()]
            .get_element_mut(gen_idx.generation())
            .expect("Tried to access element with outdated generation")
    }

    /// Returns a reference to the element at the given index,
    /// or [`None`] if the index:
    /// - Refers to a location that is currently freed.
    /// - Does not have the same generation as the element.
    ///
    /// # Panics
    /// If the index is out of bounds (in which case it belongs
    /// to a different vector).
    pub fn get_element(&self, gen_idx: GenerationalIdx) -> Option<&T> {
        if self.free_list.contains(&gen_idx.idx()) {
            None
        } else {
            self.elements[gen_idx.idx()].get_element(gen_idx.generation())
        }
    }

    /// Returns a mutable reference to the element at the given index,
    /// or [`None`] if the index:
    /// - Refers to a location that is currently freed.
    /// - Does not have the same generation as the element.
    ///
    /// # Panics
    /// If the index is out of bounds (in which case it belongs
    /// to a different vector).
    pub fn get_element_mut(&mut self, gen_idx: GenerationalIdx) -> Option<&mut T> {
        if self.free_list.contains(&gen_idx.idx()) {
            None
        } else {
            self.elements[gen_idx.idx()].get_element_mut(gen_idx.generation())
        }
    }

    /// Inserts the given element into the vector. If a freed
    /// location is available, this is used, otherwise the vector
    /// is grown in length and the element inserted at the end.
    ///
    /// If the element is inserted at a freed location, the
    /// generation of this location is advanced. This makes it
    /// impossible for (now invalidated) previous indices into
    /// the same location to access the new element.
    ///
    /// # Returns
    /// The index where the element was added.
    pub fn add_element(&mut self, element: T) -> GenerationalIdx {
        if let Some(free_idx) = self.free_list.pop_front() {
            let generation = self.elements[free_idx].update_and_advance_generation(element);
            GenerationalIdx::new(generation, free_idx)
        } else {
            let idx = GenerationalIdx::new_first_generation(self.elements.len());
            self.elements
                .push(GenerationalElement::new_first_generation(element));
            idx
        }
    }

    /// Removes the element at the given index. The underlying
    /// [`Vec`] is not modified, instead the index is registered
    /// as free.
    ///
    /// # Panics
    /// If the index:
    /// - Refers to a location that is currently freed.
    /// - Is out of bounds (in which case it belongs
    ///   to a different vector).
    pub fn free_element_at_idx(&mut self, gen_idx: GenerationalIdx) {
        let idx = gen_idx.idx();
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

    /// Remove all elements by registering every occupied index as free.
    pub fn free_all_elements(&mut self) {
        self.free_list.clear();
        self.free_list.extend(0..self.elements.len());
    }
}

impl GenerationalIdx {
    fn new(generation: Generation, idx: usize) -> Self {
        Self { generation, idx }
    }

    fn new_first_generation(idx: usize) -> Self {
        Self::new(0, idx)
    }

    fn idx(&self) -> usize {
        self.idx
    }

    fn generation(&self) -> Generation {
        self.generation
    }
}

impl Ord for GenerationalIdx {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.idx.cmp(&other.idx)
    }
}

impl PartialOrd for GenerationalIdx {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> GenerationalElement<T> {
    fn new_first_generation(element: T) -> Self {
        Self {
            generation: 0,
            element,
        }
    }

    fn get_element(&self, generation: Generation) -> Option<&T> {
        if generation == self.generation {
            Some(&self.element)
        } else {
            None
        }
    }

    fn get_element_mut(&mut self, generation: Generation) -> Option<&mut T> {
        if generation == self.generation {
            Some(&mut self.element)
        } else {
            None
        }
    }

    fn update_and_advance_generation(&mut self, element: T) -> Generation {
        self.element = element;
        self.generation += 1;
        self.generation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creating_vec_works() {
        let vec = GenerationalReusingVec::<f32>::new();
        assert_eq!(vec.n_elements(), 0);
    }

    #[test]
    #[should_panic]
    fn demanding_element_from_empty_vec_fails() {
        let vec = GenerationalReusingVec::<f32>::new();
        vec.element(GenerationalIdx::new_first_generation(0));
    }

    #[test]
    #[should_panic]
    fn demanding_mutable_element_from_empty_vec_fails() {
        let mut vec = GenerationalReusingVec::<f32>::new();
        vec.element_mut(GenerationalIdx::new_first_generation(0));
    }

    #[test]
    #[should_panic]
    fn requesting_element_from_empty_vec_fails() {
        let vec = GenerationalReusingVec::<f32>::new();
        vec.get_element(GenerationalIdx::new_first_generation(0));
    }

    #[test]
    #[should_panic]
    fn requesting_mutable_element_from_empty_vec_fails() {
        let mut vec = GenerationalReusingVec::<f32>::new();
        vec.get_element_mut(GenerationalIdx::new_first_generation(0));
    }

    #[test]
    fn adding_to_empty_vec_works() {
        let mut vec = GenerationalReusingVec::<f32>::new();
        let gen_idx = vec.add_element(1.0);

        assert_eq!(gen_idx.idx(), 0);
        assert_eq!(gen_idx.generation(), 0);

        assert_eq!(vec.n_elements(), 1);

        assert_eq!(*vec.element(gen_idx), 1.0);
        assert_eq!(*vec.element_mut(gen_idx), 1.0);

        assert_eq!(vec.get_element(gen_idx), Some(&1.0));
        assert_eq!(vec.get_element_mut(gen_idx), Some(&mut 1.0));
    }

    #[test]
    #[should_panic]
    fn freeing_out_of_bounds_idx_fails() {
        let mut vec = GenerationalReusingVec::<f32>::new();
        vec.free_element_at_idx(GenerationalIdx::new_first_generation(0));
    }

    #[test]
    fn freeing_only_element_in_vec_works() {
        let mut vec = GenerationalReusingVec::<f32>::new();
        let gen_idx = vec.add_element(1.0);
        vec.free_element_at_idx(gen_idx);

        assert_eq!(vec.n_elements(), 0);
        assert_eq!(vec.get_element(gen_idx), None);
        assert_eq!(vec.get_element_mut(gen_idx), None);
    }

    #[test]
    #[should_panic]
    fn demanding_freed_element_fails() {
        let mut vec = GenerationalReusingVec::<f32>::new();
        let gen_idx = vec.add_element(1.0);
        vec.free_element_at_idx(gen_idx);
        vec.element(gen_idx);
    }

    #[test]
    #[should_panic]
    fn demanding_freed_mutable_element_fails() {
        let mut vec = GenerationalReusingVec::<f32>::new();
        let gen_idx = vec.add_element(1.0);
        vec.free_element_at_idx(gen_idx);
        vec.element_mut(gen_idx);
    }

    #[test]
    fn requesting_freed_element_gives_none() {
        let mut vec = GenerationalReusingVec::<f32>::new();
        let gen_idx = vec.add_element(1.0);
        vec.free_element_at_idx(gen_idx);
        assert!(vec.get_element(gen_idx).is_none());
    }

    #[test]
    fn requesting_freed_mutable_element_gives_none() {
        let mut vec = GenerationalReusingVec::<f32>::new();
        let gen_idx = vec.add_element(1.0);
        vec.free_element_at_idx(gen_idx);
        assert!(vec.get_element_mut(gen_idx).is_none());
    }

    #[test]
    #[should_panic]
    fn freeing_freed_element_fails() {
        let mut vec = GenerationalReusingVec::<f32>::new();
        let gen_idx = vec.add_element(1.0);
        vec.free_element_at_idx(gen_idx);
        vec.free_element_at_idx(gen_idx);
    }

    #[test]
    fn adding_after_free_uses_free_location() {
        let mut vec = GenerationalReusingVec::<f32>::new();
        let first_idx = vec.add_element(0.0);
        assert_eq!(first_idx.idx(), 0);
        assert_eq!(first_idx.generation(), 0);
        assert_eq!(vec.n_elements(), 1);
        assert_eq!(*vec.element(first_idx), 0.0);

        let second_idx = vec.add_element(1.0);
        assert_eq!(second_idx.idx(), 1);
        assert_eq!(second_idx.generation(), 0);
        assert_eq!(vec.n_elements(), 2);
        assert_eq!(*vec.element(second_idx), 1.0);

        vec.free_element_at_idx(first_idx);
        assert_eq!(vec.n_elements(), 1);

        let third_idx = vec.add_element(2.0);
        assert_eq!(third_idx.idx(), first_idx.idx());
        assert_eq!(third_idx.generation(), 1);
        assert_eq!(vec.n_elements(), 2);
        assert_eq!(*vec.element(third_idx), 2.0);

        let fourth_idx = vec.add_element(3.0);
        assert_eq!(fourth_idx.idx(), 2);
        assert_eq!(fourth_idx.generation(), 0);
        assert_eq!(vec.n_elements(), 3);
        assert_eq!(*vec.element(fourth_idx), 3.0);
    }

    #[test]
    #[should_panic]
    fn demanding_element_with_outdated_generation_fails() {
        let mut vec = GenerationalReusingVec::<f32>::new();
        let first_idx = vec.add_element(0.0);
        vec.free_element_at_idx(first_idx);
        vec.add_element(1.0);
        vec.element(first_idx);
    }

    #[test]
    #[should_panic]
    fn demanding_mutable_element_with_outdated_generation_fails() {
        let mut vec = GenerationalReusingVec::<f32>::new();
        let first_idx = vec.add_element(0.0);
        vec.free_element_at_idx(first_idx);
        vec.add_element(1.0);
        vec.element_mut(first_idx);
    }

    #[test]
    fn requesting_element_with_outdated_generation_gives_none() {
        let mut vec = GenerationalReusingVec::<f32>::new();
        let first_idx = vec.add_element(0.0);
        vec.free_element_at_idx(first_idx);
        vec.add_element(1.0);
        assert!(vec.get_element(first_idx).is_none());
    }

    #[test]
    fn requesting_mutable_element_with_outdated_generation_gives_none() {
        let mut vec = GenerationalReusingVec::<f32>::new();
        let first_idx = vec.add_element(0.0);
        vec.free_element_at_idx(first_idx);
        vec.add_element(1.0);
        assert!(vec.get_element_mut(first_idx).is_none());
    }

    #[test]
    fn freeing_all_elements_for_empty_vec_works() {
        let mut vec = GenerationalReusingVec::<f32>::new();
        vec.free_all_elements();
        assert_eq!(vec.n_elements(), 0);
    }

    #[test]
    fn freeing_all_elements_for_single_element_vec_works() {
        let mut vec = GenerationalReusingVec::<f32>::new();
        let first_idx = vec.add_element(0.0);
        vec.free_all_elements();
        assert_eq!(vec.n_elements(), 0);
        assert!(vec.get_element(first_idx).is_none());
    }

    #[test]
    fn freeing_all_elements_for_multi_element_vec_works() {
        let mut vec = GenerationalReusingVec::<f32>::new();
        let first_idx = vec.add_element(0.0);
        let second_idx = vec.add_element(1.0);
        vec.free_all_elements();
        assert_eq!(vec.n_elements(), 0);
        assert!(vec.get_element(first_idx).is_none());
        assert!(vec.get_element(second_idx).is_none());
    }

    #[test]
    fn reusing_vec_after_freeing_all_elements_works() {
        let mut vec = GenerationalReusingVec::<f32>::new();
        let idx_before_free = vec.add_element(0.0);
        vec.free_all_elements();
        let idx_after_free = vec.add_element(1.0);
        assert_eq!(vec.n_elements(), 1);
        assert!(vec.get_element(idx_before_free).is_none());
        assert_eq!(*vec.element(idx_after_free), 1.0);
    }
}
