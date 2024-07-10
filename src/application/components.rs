//! Management of [`Component`](impact_ecs::component::Component)s in an
//! application.

use crate::{
    camera, component::ComponentRegistry, control, light, material, mesh, physics, scene, voxel,
};
use anyhow::Result;

/// Registers all components in the given registry.
pub fn register_all_components(registry: &mut ComponentRegistry) -> Result<()> {
    control::components::register_control_components(registry)?;
    physics::components::register_physics_components(registry)?;
    scene::components::register_scene_graph_components(registry)?;
    camera::components::register_camera_components(registry)?;
    light::components::register_light_components(registry)?;
    mesh::components::register_mesh_components(registry)?;
    material::components::register_material_components(registry)?;
    voxel::components::register_voxel_components(registry)
}
