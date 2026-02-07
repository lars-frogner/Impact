//! IDs for entities in the Impact engine.

use anyhow::{Result, anyhow};
use bytemuck::{Pod, Zeroable};
use impact_containers::{NoHashSet, nohash_hasher};
use impact_math::hash::Hash64;
use std::{
    array, fmt,
    hash::{self, Hash},
};

/// Unique ID identifying an entity.
#[roc_integration::roc(
    category = "primitive",
    package = "pf",
    module = "Entity",
    name = "Id",
    postfix = "_id"
)]
#[repr(C)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Zeroable, Pod)]
pub struct EntityID(u64);

/// Manages provision and registration of [`EntityID`]s.
#[derive(Debug)]
pub struct EntityIDManager {
    ids_in_use: NoHashSet<u64>,
    id_counter: u64,
}

impl EntityID {
    /// Hashes the given string into an entity ID.
    pub const fn hashed_from_str(input: &str) -> Self {
        Self(Hash64::from_str(input).to_u64())
    }

    /// Converts the given `u64` into an entity ID. Should only be called
    /// with values returned from [`Self::as_u64`].
    pub const fn from_u64(value: u64) -> Self {
        Self(value)
    }

    /// Returns the `u64` value corresponding to the entity ID.
    pub const fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Hash for EntityID {
    fn hash<H: hash::Hasher>(&self, hasher: &mut H) {
        hasher.write_u64(self.0);
    }
}

impl nohash_hasher::IsEnabled for EntityID {}

impl fmt::Display for EntityID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_u64())
    }
}

impl EntityIDManager {
    pub fn new() -> Self {
        Self {
            ids_in_use: NoHashSet::default(),
            id_counter: 0,
        }
    }

    /// Returns a unique entity ID.
    pub fn provide_id(&mut self) -> EntityID {
        while self.ids_in_use.contains(&self.id_counter) {
            self.id_counter += 1;
        }
        self.ids_in_use.insert(self.id_counter);
        EntityID(self.id_counter)
    }

    /// Returns an iterator over `count` unique entity IDs.
    pub fn provide_ids(&mut self, count: usize) -> impl Iterator<Item = EntityID> {
        (0..count).map(|_| self.provide_id())
    }

    /// Returns an array of `N` unique entity IDs.
    pub fn provide_id_arr<const N: usize>(&mut self) -> [EntityID; N] {
        array::from_fn(|_| self.provide_id())
    }

    /// Returns a vector of `count` unique entity IDs.
    pub fn provide_id_vec(&mut self, count: usize) -> Vec<EntityID> {
        let mut ids = Vec::with_capacity(count);
        ids.extend(self.provide_ids(count));
        ids
    }

    /// Marks the given entity ID as in use.
    ///
    /// # Errors
    /// Returns an error if the ID is already in use.
    pub fn register_id(&mut self, id: EntityID) -> Result<()> {
        let inserted = self.ids_in_use.insert(id.0);
        if inserted {
            Ok(())
        } else {
            Err(anyhow!("Entity ID {id} is already in use"))
        }
    }

    /// Marks the given entity ID as no longer in use.
    pub fn unregister_id(&mut self, id: EntityID) {
        self.ids_in_use.remove(&id.0);
    }
}
