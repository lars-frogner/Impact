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
/// [`Roc`](roc_integration::Roc) or [`RocPod`](roc_integration::RocPod) trait)
/// into a hash set. The type IDs of component types that implement
/// [`SetupComponent`](impact_ecs::component::SetupComponent) are additionally
/// added to a second hash set.
#[cfg(feature = "roc_codegen")]
pub fn gather_roc_type_ids_for_all_components() -> (
    impact_containers::HashSet<roc_integration::RocTypeID>,
    impact_containers::HashSet<roc_integration::RocTypeID>,
) {
    let mut components = impact_containers::HashSet::default();
    let mut setup_components = impact_containers::HashSet::default();
    for descriptor in inventory::iter::<ComponentDescriptor>() {
        let type_id = roc_integration::RocTypeID::from_u64(descriptor.id.as_u64());
        components.insert(type_id);
        if descriptor.category == impact_ecs::component::ComponentCategory::Setup {
            setup_components.insert(type_id);
        }
    }
    (components, setup_components)
}
