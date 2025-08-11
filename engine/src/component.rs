//! Management of [`Component`](impact_ecs::component::Component)s.

use anyhow::Result;
use impact_ecs::{
    component::{ComponentDescriptor, ComponentFlagDeclaration},
    metadata::ComponentMetadataRegistry,
};

/// Registers metadata for all components in the given registry.
pub fn register_metadata_for_all_components(
    registry: &mut ComponentMetadataRegistry,
) -> Result<()> {
    for descriptor in inventory::iter::<ComponentDescriptor> {
        registry.add_component(descriptor)?;
    }
    for declaration in inventory::iter::<ComponentFlagDeclaration> {
        registry.set_flags_for_component(declaration.id, declaration.flags)?;
    }
    Ok(())
}

/// Finds all types that have derived the
/// [`Component`](impact_ecs::component::Component) trait and gathers their
/// component IDs (which are identical to their Roc type ID if they derive the
/// [`Roc`](roc_integration::Roc) or [`RocPod`](roc_integration::RocPod)
/// trait) into a hash set.
#[cfg(feature = "roc_codegen")]
pub fn gather_roc_type_ids_for_all_components()
-> impact_containers::HashSet<roc_integration::RocTypeID> {
    inventory::iter::<ComponentDescriptor>()
        .map(|descriptor| roc_integration::RocTypeID::from_u64(descriptor.id.as_u64()))
        .collect()
}
