//! Tracking of changes to entities and collections.

/// Tracker for whether an entity has changed.
#[derive(Clone, Copy, Debug)]
pub struct EntityChangeTracker {
    changed: bool,
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
    pub fn new() -> Self {
        Self { changed: false }
    }

    /// Whether the tracker has registered a change.
    pub fn changed(&self) -> bool {
        self.changed
    }

    /// Informs the tracker that the entity has changed.
    pub fn notify_change(&mut self) {
        self.changed = true;
    }

    /// Creates a tracker with the changes in this and the given
    /// tracker merged.
    pub fn merged(&self, other: Self) -> Self {
        Self {
            changed: self.changed() || other.changed(),
        }
    }

    /// Resets the changes registered by the tracker.
    pub fn reset(&mut self) {
        self.changed = false;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_change_tracker_tracks_changes() {
        let mut tracker = EntityChangeTracker::new();
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
        assert!(!EntityChangeTracker { changed: false }
            .merged(EntityChangeTracker { changed: false })
            .changed());
        assert!(EntityChangeTracker { changed: true }
            .merged(EntityChangeTracker { changed: false })
            .changed());
        assert!(EntityChangeTracker { changed: false }
            .merged(EntityChangeTracker { changed: true })
            .changed());
        assert!(EntityChangeTracker { changed: true }
            .merged(EntityChangeTracker { changed: true })
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
