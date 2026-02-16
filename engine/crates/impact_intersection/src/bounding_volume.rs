//! Bounding volumes.

pub mod hierarchy;

use anyhow::{Result, bail};
use bytemuck::{Pod, Zeroable};
use impact_containers::NoHashMap;
use impact_geometry::AxisAlignedBoxC;
use impact_id::define_entity_id_newtype;
use roc_integration::roc;

define_entity_id_newtype! {
    /// Identifier for a [`BoundingVolume`] in a [`BoundingVolumeManager`].
    [pub] BoundingVolumeID
}

define_component_type! {
    /// Marks that an entity has a bounding volume identified by a
    /// [`BoundingVolumeID`].
    ///
    /// Use [`BoundingVolumeID::from_entity_id`] to obtain the bounding volume
    /// ID from the entity ID.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct HasBoundingVolume;
}

#[derive(Debug)]
pub struct BoundingVolumeManager {
    aabbs: NoHashMap<BoundingVolumeID, AxisAlignedBoxC>,
}

impl BoundingVolumeManager {
    pub fn new() -> Self {
        Self {
            aabbs: NoHashMap::default(),
        }
    }

    /// Returns a reference to the bounding volume with the given ID, or
    /// [`None`] if it does not exist.
    pub fn get_bounding_volume(&self, id: BoundingVolumeID) -> Option<&AxisAlignedBoxC> {
        self.aabbs.get(&id)
    }

    /// Returns a mutable reference to the bounding volume with the given ID, or
    /// [`None`] if it does not exist.
    pub fn get_bounding_volume_mut(
        &mut self,
        id: BoundingVolumeID,
    ) -> Option<&mut AxisAlignedBoxC> {
        self.aabbs.get_mut(&id)
    }

    /// Returns an iterator over all bounding volumes.
    pub fn bounding_volumes(&self) -> impl Iterator<Item = &AxisAlignedBoxC> {
        self.aabbs.values()
    }

    /// Adds the given bounding volume to the map under the given ID.
    ///
    /// # Errors
    /// Returns an error if the given bounding volume ID already exists.
    pub fn insert_bounding_volume(
        &mut self,
        id: BoundingVolumeID,
        aabb: AxisAlignedBoxC,
    ) -> Result<()> {
        if self.aabbs.contains_key(&id) {
            bail!("A bounding volume with ID {id} already exists");
        }
        self.aabbs.insert(id, aabb);
        Ok(())
    }

    /// Removes the bounding volume with the given ID from the map if it
    /// exists.
    pub fn remove_bounding_volume(&mut self, id: BoundingVolumeID) {
        self.aabbs.remove(&id);
    }

    /// Removes all bounding volumes.
    pub fn remove_all_bounding_volumes(&mut self) {
        self.aabbs.clear();
    }
}
