use std::collections::LinkedList;

#[derive(Clone, Debug, Default)]
pub struct VecWithFreeList<T> {
    elements: Vec<T>,
    free_list: LinkedList<usize>,
}

impl<T> VecWithFreeList<T> {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            free_list: LinkedList::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            elements: Vec::with_capacity(capacity),
            free_list: LinkedList::new(),
        }
    }

    pub fn n_elements(&self) -> usize {
        self.elements.len() - self.free_list.len()
    }

    pub fn element(&self, idx: usize) -> &T {
        assert!(
            !self.free_list.contains(&idx),
            "Tried to access element at vacant index"
        );
        &self.elements[idx]
    }

    pub fn element_mut(&mut self, idx: usize) -> &mut T {
        assert!(
            !self.free_list.contains(&idx),
            "Tried to access element at vacant index"
        );
        &mut self.elements[idx]
    }

    pub fn get_element(&self, idx: usize) -> Option<&T> {
        if idx >= self.elements.len() || self.free_list.contains(&idx) {
            None
        } else {
            Some(&self.elements[idx])
        }
    }

    pub fn get_element_mut(&mut self, idx: usize) -> Option<&mut T> {
        if idx >= self.elements.len() || self.free_list.contains(&idx) {
            None
        } else {
            Some(&mut self.elements[idx])
        }
    }

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
