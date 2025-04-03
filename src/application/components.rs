//! Management of [`Component`](impact_ecs::component::Component)s in an
//! application.

use crate::component::ComponentRegistry;
use anyhow::Result;
use impact_ecs::component::ComponentDescriptor;

/// Registers all components in the given registry.
pub fn register_all_components(registry: &mut ComponentRegistry) -> Result<()> {
    for descriptor in inventory::iter::<ComponentDescriptor> {
        registry.add_component(descriptor)?;
    }
    Ok(())
}
