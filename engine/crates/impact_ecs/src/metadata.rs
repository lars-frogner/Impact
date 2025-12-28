//! Management of metadata for [`Component`](crate::component::Component)s.

use crate::component::{ComponentCategory, ComponentDescriptor, ComponentFlags, ComponentID};
use anyhow::{Result, anyhow, bail};
use impact_containers::{NoHashMap, hash_map::Entry};

/// Registry for holding metadata about all
/// [`Component`](crate::component::Component)s.
#[derive(Debug)]
pub struct ComponentMetadataRegistry {
    components: NoHashMap<ComponentID, ComponentMetadata>,
}

/// Metadata for a type implementing [`Component`](crate::component::Component).
#[derive(Debug)]
pub struct ComponentMetadata {
    /// The name of the component type.
    pub name: &'static str,
    /// The category of the component type.
    pub category: ComponentCategory,
    /// Flags for the component type.
    pub flags: ComponentFlags,
}

impl ComponentMetadataRegistry {
    /// Creates a new empty component registry.
    pub fn new() -> Self {
        Self {
            components: NoHashMap::default(),
        }
    }

    /// Adds metadata for the component with the given descriptor to the
    /// registry.
    ///
    /// # Errors
    /// Returns an error if a component with the same ID is already present.
    pub fn add_component(&mut self, descriptor: &ComponentDescriptor) -> Result<()> {
        match self.components.entry(descriptor.id) {
            Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(ComponentMetadata {
                    name: descriptor.name,
                    category: descriptor.category,
                    flags: ComponentFlags::empty(),
                });
            }
            Entry::Occupied(_) => {
                bail!("Tried to add component to metadata registry twice");
            }
        }
        Ok(())
    }

    /// Sets the given flags for the component with the given ID.
    ///
    /// # Errors
    /// Returns an error if no component with the given ID is registered.
    pub fn set_flags_for_component(
        &mut self,
        id: ComponentID,
        flags: ComponentFlags,
    ) -> Result<()> {
        let component = self
            .components
            .get_mut(&id)
            .ok_or_else(|| anyhow!("Tried to access missing component in metadata registry"))?;

        component.flags |= flags;

        Ok(())
    }

    /// Returns a reference to the entry for the component with the given ID.
    ///
    /// # Panics
    /// If no component with the given ID is registered.
    pub fn metadata(&self, component_id: ComponentID) -> &ComponentMetadata {
        self.components
            .get(&component_id)
            .expect("Tried to access metadata for missing component in metadata registry")
    }
}

impl Default for ComponentMetadataRegistry {
    fn default() -> Self {
        Self::new()
    }
}
