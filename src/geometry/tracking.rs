//! Tracking of changes to entities and collections.

use std::sync::atomic::{AtomicBool, Ordering};

/// Atomic tracker for whether an entity has changed.
#[derive(Debug)]
pub struct EntityChangeTracker {
    changed: AtomicBool,
}

/// Tracker for how a collection has changed.
#[derive(Clone, Copy, Debug)]
pub struct CollectionChangeTracker {
    change: CollectionChange,
}

/// In what way a collection has changed.
#[derive(Copy, Clone, Debug, PartialEq)]
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
    pub fn new() -> Self {
        Self {
            change: CollectionChange::None,
        }
    }

    /// What kind of change the tracker has registered.
    pub fn change(&self) -> CollectionChange {
        self.change
    }

    /// Informs the tracker that the contents of the
    /// collection have changed.
    ///
    /// If the number of elements has already changed,
    /// we ignore this since it already implies that the
    /// contents have changed.
    pub fn notify_content_change(&mut self) {
        if self.change != CollectionChange::Count {
            self.change = CollectionChange::Contents;
        }
    }

    /// Informs the tracker that the number of elements in the
    /// collection has changed.
    pub fn notify_count_change(&mut self) {
        self.change = CollectionChange::Count;
    }

    /// Creates a tracker with the changes in this and the given
    /// tracker merged.
    pub fn merged(&self, other: Self) -> Self {
        Self {
            change: match (self.change(), other.change()) {
                (CollectionChange::Count, _) | (_, CollectionChange::Count) => {
                    CollectionChange::Count
                }
                (CollectionChange::Contents, _) | (_, CollectionChange::Contents) => {
                    CollectionChange::Contents
                }
                _ => CollectionChange::None,
            },
        }
    }

    /// Resets the changes registered by the tracker.
    pub fn reset(&mut self) {
        self.change = CollectionChange::None;
    }
}

impl Default for CollectionChangeTracker {
    fn default() -> Self {
        Self::new()
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
        let mut tracker = CollectionChangeTracker::new();
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
        let mut tracker = CollectionChangeTracker::new();
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
            CollectionChangeTracker {
                change: CollectionChange::None
            }
            .merged(CollectionChangeTracker {
                change: CollectionChange::None
            })
            .change(),
            CollectionChange::None
        );
        assert_eq!(
            CollectionChangeTracker {
                change: CollectionChange::Contents
            }
            .merged(CollectionChangeTracker {
                change: CollectionChange::None
            })
            .change(),
            CollectionChange::Contents
        );
        assert_eq!(
            CollectionChangeTracker {
                change: CollectionChange::Contents
            }
            .merged(CollectionChangeTracker {
                change: CollectionChange::Contents
            })
            .change(),
            CollectionChange::Contents
        );
        assert_eq!(
            CollectionChangeTracker {
                change: CollectionChange::Count
            }
            .merged(CollectionChangeTracker {
                change: CollectionChange::None
            })
            .change(),
            CollectionChange::Count
        );
        assert_eq!(
            CollectionChangeTracker {
                change: CollectionChange::Count
            }
            .merged(CollectionChangeTracker {
                change: CollectionChange::Contents
            })
            .change(),
            CollectionChange::Count
        );
        assert_eq!(
            CollectionChangeTracker {
                change: CollectionChange::Count
            }
            .merged(CollectionChangeTracker {
                change: CollectionChange::Count
            })
            .change(),
            CollectionChange::Count
        );
    }
}
