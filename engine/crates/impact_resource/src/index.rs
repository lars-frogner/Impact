//! Resource indexing for mapping persistent IDs to handles.

use crate::{ResourceLabelProvider, ResourcePID};
use impact_containers::HashMap;
use std::{fmt, hash::Hash};

/// An index that maps persistent resource IDs to handles and vice versa.
#[derive(Clone, Debug)]
pub struct ResourceIndex<PID, H> {
    pid_to_handle: HashMap<PID, H>,
    handle_to_pid: HashMap<H, PID>,
}

impl<PID, H> ResourceIndex<PID, H>
where
    PID: ResourcePID,
    H: Copy + Eq + Hash,
{
    /// Creates a new empty resource index.
    pub fn new() -> Self {
        Self {
            pid_to_handle: HashMap::default(),
            handle_to_pid: HashMap::default(),
        }
    }

    /// Returns the handle associated with the given persistent ID.
    pub fn get_handle(&self, pid: PID) -> Option<H> {
        self.pid_to_handle.get(&pid).copied()
    }

    /// Returns the persistent ID associated with the given handle.
    pub fn get_pid(&self, handle: H) -> Option<PID> {
        self.handle_to_pid.get(&handle).copied()
    }

    /// Associates a persistent ID with a handle, overwriting any existing
    /// binding involving the ID or handle.
    pub fn bind(&mut self, pid: PID, handle: H) {
        if let Some(old_handle) = self.pid_to_handle.insert(pid, handle) {
            self.handle_to_pid.remove(&old_handle);
        }
        if let Some(old_pid) = self.handle_to_pid.insert(handle, pid) {
            self.pid_to_handle.remove(&old_pid);
        }
    }

    /// Removes the binding for the given handle.
    pub fn unbind_by_handle(&mut self, handle: H) {
        if let Some(pid) = self.handle_to_pid.remove(&handle) {
            self.pid_to_handle.remove(&pid);
        }
    }

    /// Removes the binding for the given persistent ID.
    pub fn unbind_by_pid(&mut self, pid: PID) {
        if let Some(h) = self.pid_to_handle.remove(&pid) {
            self.handle_to_pid.remove(&h);
        }
    }
}

impl<PID, H> Default for ResourceIndex<PID, H>
where
    PID: ResourcePID,
    H: Copy + Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<PID, H> ResourceLabelProvider<H> for ResourceIndex<PID, H>
where
    PID: ResourcePID,
    H: Copy + Eq + Hash + fmt::Display,
{
    fn create_label(&self, handle: H) -> String {
        self.get_pid(handle)
            .map_or_else(|| handle.to_string(), |pid| pid.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::Zeroable;
    use impact_containers::SlotKey;

    // Test PID type
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    struct TestPID(u32);

    impl fmt::Display for TestPID {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "TestPID({})", self.0)
        }
    }

    impl ResourcePID for TestPID {}

    // Test handle type
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    struct TestHandle(SlotKey);

    impl fmt::Display for TestHandle {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "TestHandle({})", self.0)
        }
    }

    fn test_handle(id: u32) -> TestHandle {
        let mut key = SlotKey::zeroed();
        let bytes = bytemuck::bytes_of_mut(&mut key);
        bytes[0] = id as u8;
        TestHandle(key)
    }

    #[test]
    fn getting_from_empty_index_returns_none() {
        let index = ResourceIndex::<TestPID, TestHandle>::new();
        let pid1 = TestPID(1);
        let handle1 = test_handle(1);

        assert!(index.get_handle(pid1).is_none());
        assert!(index.get_pid(handle1).is_none());
    }

    #[test]
    fn binding_pid_to_handle_allows_lookup_in_both_directions() {
        let mut index = ResourceIndex::new();
        let pid1 = TestPID(1);
        let handle1 = test_handle(1);

        index.bind(pid1, handle1);

        assert_eq!(index.get_handle(pid1), Some(handle1));
        assert_eq!(index.get_pid(handle1), Some(pid1));
    }

    #[test]
    fn binding_multiple_pairs_works_correctly() {
        let mut index = ResourceIndex::new();
        let pid1 = TestPID(1);
        let pid2 = TestPID(2);
        let handle1 = test_handle(1);
        let handle2 = test_handle(2);

        index.bind(pid1, handle1);
        index.bind(pid2, handle2);

        assert_eq!(index.get_handle(pid1), Some(handle1));
        assert_eq!(index.get_handle(pid2), Some(handle2));
        assert_eq!(index.get_pid(handle1), Some(pid1));
        assert_eq!(index.get_pid(handle2), Some(pid2));
    }

    #[test]
    fn binding_same_pid_to_different_handle_overwrites_previous_binding() {
        let mut index = ResourceIndex::new();
        let pid1 = TestPID(1);
        let handle1 = test_handle(1);
        let handle2 = test_handle(2);

        index.bind(pid1, handle1);
        index.bind(pid1, handle2);

        assert_eq!(index.get_handle(pid1), Some(handle2));
        assert_eq!(index.get_pid(handle2), Some(pid1));
        assert!(index.get_pid(handle1).is_none());
    }

    #[test]
    fn binding_different_pid_to_same_handle_overwrites_previous_binding() {
        let mut index = ResourceIndex::new();
        let pid1 = TestPID(1);
        let pid2 = TestPID(2);
        let handle1 = test_handle(1);

        index.bind(pid1, handle1);
        index.bind(pid2, handle1);

        assert_eq!(index.get_handle(pid2), Some(handle1));
        assert_eq!(index.get_pid(handle1), Some(pid2));
        assert!(index.get_handle(pid1).is_none());
    }

    #[test]
    fn unbinding_by_handle_removes_both_directions() {
        let mut index = ResourceIndex::new();
        let pid1 = TestPID(1);
        let handle1 = test_handle(1);
        index.bind(pid1, handle1);

        index.unbind_by_handle(handle1);

        assert!(index.get_handle(pid1).is_none());
        assert!(index.get_pid(handle1).is_none());
    }

    #[test]
    fn unbinding_by_pid_removes_both_directions() {
        let mut index = ResourceIndex::new();
        let pid1 = TestPID(1);
        let handle1 = test_handle(1);
        index.bind(pid1, handle1);

        index.unbind_by_pid(pid1);

        assert!(index.get_handle(pid1).is_none());
        assert!(index.get_pid(handle1).is_none());
    }

    #[test]
    fn unbinding_nonexistent_entries_does_nothing() {
        let mut index = ResourceIndex::new();
        let pid1 = TestPID(1);
        let handle1 = test_handle(1);
        index.bind(pid1, handle1);

        index.unbind_by_handle(test_handle(99));
        index.unbind_by_pid(TestPID(99));

        assert_eq!(index.get_handle(pid1), Some(handle1));
        assert_eq!(index.get_pid(handle1), Some(pid1));
    }
}
