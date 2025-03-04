//! Management of [`Component`](impact_ecs::component::Component)s.

use anyhow::{Result, bail};
use impact_ecs::component::ComponentID;
use std::collections::{HashMap, hash_map::Entry};

/// Registry for holding metadata about all
/// [`Component`](impact_ecs::component::Component)s.
#[derive(Debug)]
pub struct ComponentRegistry {
    components: HashMap<ComponentID, ComponentEntry>,
}

/// An entry in the [`ComponentRegistry`].
#[derive(Debug)]
pub struct ComponentEntry {
    /// The name of the component.
    pub name: &'static str,
    /// The category of the component.
    pub category: ComponentCategory,
}

/// The category of a component.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ComponentCategory {
    /// A persistent component whose current state is always reflected in the
    /// world.
    Standard,
    /// A helper component used for creating entities, which is no longer
    /// present in the entity after it has been created.
    Setup,
}

impl ComponentRegistry {
    /// Creates a new empty component registry.
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
        }
    }

    /// Adds an entry for the component with the given ID and name to the
    /// registry.
    ///
    /// # Errors
    /// Returns an error if a component with the same ID is already present.
    pub fn add_component(
        &mut self,
        id: ComponentID,
        name: &'static str,
        category: ComponentCategory,
    ) -> Result<()> {
        match self.components.entry(id) {
            Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(ComponentEntry { name, category });
            }
            Entry::Occupied(_) => {
                bail!("Tried to add component to registry twice");
            }
        }
        Ok(())
    }

    /// Returns a reference to the entry for the component with the given ID.
    ///
    /// # Panics
    /// If no component with the given ID is registered.
    pub fn component_with_id(&self, component_id: ComponentID) -> &ComponentEntry {
        self.components
            .get(&component_id)
            .expect("Tried to access missing component in registry")
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
