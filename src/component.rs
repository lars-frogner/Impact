//! Management of [`Component`](impact_ecs::component::Component)s.

use anyhow::{Result, bail};
use impact_ecs::component::{ComponentCategory, ComponentDescriptor, ComponentID};
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

impl ComponentRegistry {
    /// Creates a new empty component registry.
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
        }
    }

    /// Adds an entry for the component with the given descriptor to the
    /// registry.
    ///
    /// # Errors
    /// Returns an error if a component with the same ID is already present.
    pub fn add_component(&mut self, descriptor: &ComponentDescriptor) -> Result<()> {
        match self.components.entry(descriptor.id) {
            Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(ComponentEntry {
                    name: descriptor.name,
                    category: descriptor.category,
                });
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
