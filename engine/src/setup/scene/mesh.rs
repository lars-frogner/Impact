//! Setup of meshes for new entities.

use crate::resource::ResourceManager;
use anyhow::{Result, anyhow};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_material::{MaterialHandle, MaterialLibrary};
use impact_mesh::{
    TriangleMeshHandle, TriangleMeshID,
    setup::{
        self, BoxMesh, CircularFrustumMesh, ConeMesh, CylinderMesh, HemisphereMesh, RectangleMesh,
        SphereMesh, TriangleMeshTemplate,
    },
    texture_projection::PlanarTextureProjection,
};
use parking_lot::RwLock;

/// Checks if the entites-to-be with the given components have a component
/// representing a mesh, and if so, generates the meshes and adds them to the
/// resource registry if not present, then adds the appropriate mesh components
/// to the entities.
pub fn setup_meshes_for_new_entities(
    resource_manager: &RwLock<ResourceManager>,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    setup!(
        {
            let resource_manager = resource_manager.read();
        },
        components,
        |mesh_id: &TriangleMeshID| -> Result<TriangleMeshHandle> {
            resource_manager
                .triangle_meshes
                .index
                .get_handle(*mesh_id)
                .ok_or_else(|| anyhow!("Tried to create entity with missing mesh {mesh_id}"))
        },
        ![TriangleMeshHandle]
    )?;

    setup!(
        {
            let mut resource_manager = resource_manager.write();
        },
        components,
        |rectangle_mesh: &RectangleMesh,
         planar_projection: Option<&setup::PlanarTextureProjection>|
         -> TriangleMeshHandle {
            setup_triangle_mesh_for_new_entity(
                &mut resource_manager,
                TriangleMeshTemplate::Rectangle(*rectangle_mesh),
                planar_projection,
            )
        },
        ![TriangleMeshHandle]
    );

    setup!(
        {
            let mut resource_manager = resource_manager.write();
        },
        components,
        |box_mesh: &BoxMesh,
         planar_projection: Option<&setup::PlanarTextureProjection>|
         -> TriangleMeshHandle {
            setup_triangle_mesh_for_new_entity(
                &mut resource_manager,
                TriangleMeshTemplate::Box(*box_mesh),
                planar_projection,
            )
        },
        ![TriangleMeshHandle]
    );

    setup!(
        {
            let mut resource_manager = resource_manager.write();
        },
        components,
        |cylinder_mesh: &CylinderMesh,
         planar_projection: Option<&setup::PlanarTextureProjection>|
         -> TriangleMeshHandle {
            setup_triangle_mesh_for_new_entity(
                &mut resource_manager,
                TriangleMeshTemplate::Cylinder(*cylinder_mesh),
                planar_projection,
            )
        },
        ![TriangleMeshHandle]
    );

    setup!(
        {
            let mut resource_manager = resource_manager.write();
        },
        components,
        |cone_mesh: &ConeMesh,
         planar_projection: Option<&setup::PlanarTextureProjection>|
         -> TriangleMeshHandle {
            setup_triangle_mesh_for_new_entity(
                &mut resource_manager,
                TriangleMeshTemplate::Cone(*cone_mesh),
                planar_projection,
            )
        },
        ![TriangleMeshHandle]
    );

    setup!(
        {
            let mut resource_manager = resource_manager.write();
        },
        components,
        |circular_frustum_mesh: &CircularFrustumMesh,
         planar_projection: Option<&setup::PlanarTextureProjection>|
         -> TriangleMeshHandle {
            setup_triangle_mesh_for_new_entity(
                &mut resource_manager,
                TriangleMeshTemplate::CircularFrustum(*circular_frustum_mesh),
                planar_projection,
            )
        },
        ![TriangleMeshHandle]
    );

    setup!(
        {
            let mut resource_manager = resource_manager.write();
        },
        components,
        |sphere_mesh: &SphereMesh,
         planar_projection: Option<&setup::PlanarTextureProjection>|
         -> TriangleMeshHandle {
            setup_triangle_mesh_for_new_entity(
                &mut resource_manager,
                TriangleMeshTemplate::Sphere(*sphere_mesh),
                planar_projection,
            )
        },
        ![TriangleMeshHandle]
    );

    setup!(
        {
            let mut resource_manager = resource_manager.write();
        },
        components,
        |hemisphere_mesh: &HemisphereMesh,
         planar_projection: Option<&setup::PlanarTextureProjection>|
         -> TriangleMeshHandle {
            setup_triangle_mesh_for_new_entity(
                &mut resource_manager,
                TriangleMeshTemplate::Hemisphere(*hemisphere_mesh),
                planar_projection,
            )
        },
        ![TriangleMeshHandle]
    );

    Ok(())
}

fn setup_triangle_mesh_for_new_entity(
    resource_manager: &mut ResourceManager,
    template: TriangleMeshTemplate,
    planar_projection: Option<&setup::PlanarTextureProjection>,
) -> TriangleMeshHandle {
    match planar_projection {
        Some(planar_projection) => setup::setup_triangle_mesh_from_template(
            &mut resource_manager.triangle_meshes,
            &template,
            None,
            Some(&planar_projection.create()),
        ),
        None => setup::setup_triangle_mesh_from_template(
            &mut resource_manager.triangle_meshes,
            &template,
            None,
            Option::<&PlanarTextureProjection<_>>::None,
        ),
    }
}

/// Checks if the entities-to-be with the given components have a material
/// component and a component for a mesh that misses vertex attributes required
/// by the material, and if so, generates the missing vertex attributes if
/// possible.
pub fn generate_missing_vertex_properties_for_new_entity_meshes(
    resource_manager: &RwLock<ResourceManager>,
    material_library: &MaterialLibrary,
    components: &ArchetypeComponentStorage,
) {
    setup!(
        {
            let mut resource_manager = resource_manager.write();
        },
        components,
        |mesh_handle: &TriangleMeshHandle, material: &MaterialHandle| {
            let Some(material_specification) =
                material_library.get_material_specification(material.material_id())
            else {
                impact_log::warn!(
                    "Tried to generate missing vertex properties for missing material {}",
                    material.material_id()
                );
                return;
            };

            let vertex_attribute_requirements =
                material_specification.vertex_attribute_requirements();

            setup::generate_missing_vertex_properties_for_mesh(
                &mut resource_manager.triangle_meshes,
                *mesh_handle,
                vertex_attribute_requirements,
            );
        }
    );
}
