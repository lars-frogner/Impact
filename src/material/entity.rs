//! Management of material-related components for entities.

pub mod blinn_phong;
pub mod fixed;
pub mod microfacet;
mod prepass;
pub mod skybox;
pub mod vertex_color;

use crate::{
    assets::Assets,
    gpu::GraphicsDevice,
    material::MaterialLibrary,
    scene::{InstanceFeatureManager, RenderResourcesDesynchronized},
};
use impact_ecs::archetype::ArchetypeComponentStorage;
use std::sync::RwLock;

/// Checks if the entity-to-be with the given components has the components for
/// a material, and if so, adds the material specification to the material
/// library if not already present, adds the appropriate material property
/// texture set to the material library if not already present, registers the
/// material in the instance feature manager and adds the appropriate material
/// component to the entity.
pub fn add_material_component_for_entity(
    graphics_device: &GraphicsDevice,
    assets: &Assets,
    material_library: &RwLock<MaterialLibrary>,
    instance_feature_manager: &RwLock<InstanceFeatureManager>,
    components: &mut ArchetypeComponentStorage,
    desynchronized: &mut RenderResourcesDesynchronized,
) {
    vertex_color::add_vertex_color_material_component_for_entity(
        material_library,
        components,
        desynchronized,
    );

    fixed::add_fixed_color_material_component_for_entity(
        material_library,
        instance_feature_manager,
        components,
        desynchronized,
    );

    fixed::add_fixed_texture_material_component_for_entity(
        graphics_device,
        assets,
        material_library,
        components,
        desynchronized,
    );

    blinn_phong::add_blinn_phong_material_component_for_entity(
        graphics_device,
        assets,
        material_library,
        instance_feature_manager,
        components,
        desynchronized,
    );

    microfacet::add_microfacet_material_component_for_entity(
        graphics_device,
        assets,
        material_library,
        instance_feature_manager,
        components,
        desynchronized,
    );

    skybox::add_skybox_material_component_for_entity(
        graphics_device,
        assets,
        material_library,
        components,
        desynchronized,
    );
}
