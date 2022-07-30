//! Tracking of changes to entities and collections.

use atomic_enum::atomic_enum;
use std::sync::atomic::{AtomicBool, Ordering};

/// Atomic tracker for whether an entity has changed.
#[derive(Debug)]
pub struct EntityChangeTracker {
    changed: AtomicBool,
}

/// Atomic tracker for how a collection has changed.
#[derive(Debug)]
pub struct CollectionChangeTracker {
    change: AtomicCollectionChange,
}

/// In what way a collection has changed.
#[atomic_enum]
#[derive(PartialEq)]
pub enum CollectionChange {
    None,
    Contents,
    Count,
}

impl EntityChangeTracker {
    /// Creates a new tracker.
    pub fn new(changed: bool) -> Self {
        Self {
            changed: AtomicBool::new(changed),
        }
    }

    /// Whether the tracker has registered a change.
    pub fn changed(&self) -> bool {
        self.changed.load(Ordering::Acquire)
    }

    /// Informs the tracker that the entity has changed.
    pub fn notify_change(&self) {
        self.changed.store(true, Ordering::Release);
    }

    /// Creates a tracker with the changes in this and the given
    /// tracker merged.
    pub fn merged(&self, other: &Self) -> Self {
        Self {
            changed: AtomicBool::new(self.changed() || other.changed()),
        }
    }

    /// Resets the changes registered by the tracker.
    pub fn reset(&self) {
        self.changed.store(false, Ordering::Release);
    }
}

impl Default for EntityChangeTracker {
    fn default() -> Self {
        Self::new(false)
    }
}

impl CollectionChangeTracker {
    /// Creates a new tracker.
    pub fn new(change: CollectionChange) -> Self {
        Self {
            change: AtomicCollectionChange::new(change),
        }
    }

    /// What kind of change the tracker has registered.
    pub fn change(&self) -> CollectionChange {
        self.change.load(Ordering::Acquire)
    }

    /// Informs the tracker that the contents of the
    /// collection have changed.
    ///
    /// If the number of elements has already changed,
    /// we ignore this since it already implies that the
    /// contents have changed.
    pub fn notify_content_change(&self) {
        // Update `change` to `Contents` if it is `None`
        let _ = self.change.compare_exchange(
            CollectionChange::None,
            CollectionChange::Contents,
            Ordering::Acquire,
            Ordering::Relaxed,
        );
    }

    /// Informs the tracker that the number of elements in the
    /// collection has changed.
    pub fn notify_count_change(&self) {
        self.change
            .store(CollectionChange::Count, Ordering::Release);
    }

    /// Creates a tracker with the changes in this and the given
    /// tracker merged.
    pub fn merged(&self, other: &Self) -> Self {
        Self {
            change: AtomicCollectionChange::new(match (self.change(), other.change()) {
                (CollectionChange::Count, _) | (_, CollectionChange::Count) => {
                    CollectionChange::Count
                }
                (CollectionChange::Contents, _) | (_, CollectionChange::Contents) => {
                    CollectionChange::Contents
                }
                _ => CollectionChange::None,
            }),
        }
    }

    /// Resets the changes registered by the tracker.
    pub fn reset(&self) {
        self.change.store(CollectionChange::None, Ordering::Release);
    }
}

impl Default for CollectionChangeTracker {
    fn default() -> Self {
        Self::new(CollectionChange::None)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn entity_change_tracker_tracks_changes() {
        let tracker = EntityChangeTracker::default();
        assert!(
            !tracker.changed(),
            "Tracker reported change after construction"
        );
        tracker.notify_change();
        assert!(
            tracker.changed(),
            "Tracker reported no change after change notification"
        );
        tracker.reset();
        assert!(!tracker.changed(), "Tracker reported change after reset");
    }

    #[test]
    fn entity_change_tracker_merging_works() {
        assert!(!EntityChangeTracker::new(false)
            .merged(&EntityChangeTracker::new(false))
            .changed());
        assert!(EntityChangeTracker::new(true)
            .merged(&EntityChangeTracker::new(false))
            .changed());
        assert!(EntityChangeTracker::new(false)
            .merged(&EntityChangeTracker::new(true))
            .changed());
        assert!(EntityChangeTracker::new(true)
            .merged(&EntityChangeTracker::new(true))
            .changed());
    }

    #[test]
    fn collection_change_tracker_tracks_content_changes() {
        let tracker = CollectionChangeTracker::default();
        assert_eq!(
            tracker.change(),
            CollectionChange::None,
            "Tracker reported change after construction"
        );
        tracker.notify_content_change();
        assert_eq!(
            tracker.change(),
            CollectionChange::Contents,
            "Tracker reported no content change after change notification"
        );
        tracker.reset();
        assert_eq!(
            tracker.change(),
            CollectionChange::None,
            "Tracker reported change after reset"
        );
    }

    #[test]
    fn collection_change_tracker_tracks_count_changes() {
        let tracker = CollectionChangeTracker::default();
        tracker.notify_count_change();
        assert_eq!(
            tracker.change(),
            CollectionChange::Count,
            "Tracker reported no count change after change notification"
        );
    }

    #[test]
    fn collection_change_tracker_merging_works() {
        assert_eq!(
            CollectionChangeTracker::new(CollectionChange::None)
                .merged(&CollectionChangeTracker::new(CollectionChange::None))
                .change(),
            CollectionChange::None
        );
        assert_eq!(
            CollectionChangeTracker::new(CollectionChange::Contents)
                .merged(&CollectionChangeTracker::new(CollectionChange::None))
                .change(),
            CollectionChange::Contents
        );
        assert_eq!(
            CollectionChangeTracker::new(CollectionChange::Contents)
                .merged(&CollectionChangeTracker::new(CollectionChange::Contents))
                .change(),
            CollectionChange::Contents
        );
        assert_eq!(
            CollectionChangeTracker::new(CollectionChange::Count)
                .merged(&CollectionChangeTracker::new(CollectionChange::None))
                .change(),
            CollectionChange::Count
        );
        assert_eq!(
            CollectionChangeTracker::new(CollectionChange::Count)
                .merged(&CollectionChangeTracker::new(CollectionChange::Contents))
                .change(),
            CollectionChange::Count
        );
        assert_eq!(
            CollectionChangeTracker::new(CollectionChange::Count)
                .merged(&CollectionChangeTracker::new(CollectionChange::Count))
                .change(),
            CollectionChange::Count
        );
    }
}
