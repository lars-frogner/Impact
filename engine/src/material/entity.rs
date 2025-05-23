//! Management of materials for entities.

pub mod fixed;
pub mod physical;

use crate::{
    assets::Assets,
    gpu::GraphicsDevice,
    material::{MaterialLibrary, components::MaterialComp},
    model::InstanceFeatureManager,
    scene::RenderResourcesDesynchronized,
};
use anyhow::Result;
use impact_ecs::{archetype::ArchetypeComponentStorage, world::EntityEntry};
use std::sync::RwLock;

/// Checks if the entity-to-be with the given components has the components for
/// a material, and if so, adds the material specification to the material
/// library if not already present, adds the appropriate material property
/// texture set to the material library if not already present, registers the
/// material in the instance feature manager and adds the appropriate material
/// component to the entity.
pub fn setup_material_for_new_entity(
    graphics_device: &GraphicsDevice,
    assets: &Assets,
    material_library: &RwLock<MaterialLibrary>,
    instance_feature_manager: &RwLock<InstanceFeatureManager>,
    components: &mut ArchetypeComponentStorage,
    desynchronized: &mut RenderResourcesDesynchronized,
) -> Result<()> {
    fixed::setup_fixed_color_material_for_new_entity(
        material_library,
        instance_feature_manager,
        components,
        desynchronized,
    );

    fixed::setup_fixed_texture_material_for_new_entity(
        graphics_device,
        assets,
        material_library,
        components,
    )?;

    physical::setup_physical_material_for_new_entity(
        graphics_device,
        assets,
        material_library,
        instance_feature_manager,
        components,
        desynchronized,
    )?;

    Ok(())
}

/// Checks if the given entity has a [`MaterialComp`], and if so, removes the
/// assocated instance features from the given [`InstanceFeatureManager`].
pub fn cleanup_material_for_removed_entity(
    instance_feature_manager: &RwLock<InstanceFeatureManager>,
    entity: &EntityEntry<'_>,
    desynchronized: &mut RenderResourcesDesynchronized,
) {
    if let Some(material) = entity.get_component::<MaterialComp>() {
        let material = material.access();

        if let Some(feature_id) = material.material_handle().material_property_feature_id() {
            instance_feature_manager
                .write()
                .unwrap()
                .get_storage_mut_for_feature_type_id(feature_id.feature_type_id())
                .expect("Missing storage for material feature")
                .remove_feature(feature_id);
            desynchronized.set_yes();
        }
    }
}
