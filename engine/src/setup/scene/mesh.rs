//! Setup of meshes for new entities.

use crate::{lock_order::OrderedRwLock, resource::ResourceManager};
use anyhow::Result;
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_material::MaterialID;
use impact_mesh::{
    TriangleMeshID,
    setup::{
        self, BoxMesh, CapsuleMesh, CircularFrustumMesh, ConeMesh, CylinderMesh, HemisphereMesh,
        RectangleMesh, SphereMesh, TriangleMeshTemplate,
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
            let mut resource_manager = resource_manager.owrite();
        },
        components,
        |rectangle_mesh: &RectangleMesh,
         planar_projection: Option<&setup::PlanarTextureProjection>|
         -> TriangleMeshID {
            setup_triangle_mesh_for_new_entity(
                &mut resource_manager,
                TriangleMeshTemplate::Rectangle(*rectangle_mesh),
                planar_projection,
            )
        },
        ![TriangleMeshID]
    );

    setup!(
        {
            let mut resource_manager = resource_manager.owrite();
        },
        components,
        |box_mesh: &BoxMesh,
         planar_projection: Option<&setup::PlanarTextureProjection>|
         -> TriangleMeshID {
            setup_triangle_mesh_for_new_entity(
                &mut resource_manager,
                TriangleMeshTemplate::Box(*box_mesh),
                planar_projection,
            )
        },
        ![TriangleMeshID]
    );

    setup!(
        {
            let mut resource_manager = resource_manager.owrite();
        },
        components,
        |cylinder_mesh: &CylinderMesh,
         planar_projection: Option<&setup::PlanarTextureProjection>|
         -> TriangleMeshID {
            setup_triangle_mesh_for_new_entity(
                &mut resource_manager,
                TriangleMeshTemplate::Cylinder(*cylinder_mesh),
                planar_projection,
            )
        },
        ![TriangleMeshID]
    );

    setup!(
        {
            let mut resource_manager = resource_manager.owrite();
        },
        components,
        |cone_mesh: &ConeMesh,
         planar_projection: Option<&setup::PlanarTextureProjection>|
         -> TriangleMeshID {
            setup_triangle_mesh_for_new_entity(
                &mut resource_manager,
                TriangleMeshTemplate::Cone(*cone_mesh),
                planar_projection,
            )
        },
        ![TriangleMeshID]
    );

    setup!(
        {
            let mut resource_manager = resource_manager.owrite();
        },
        components,
        |circular_frustum_mesh: &CircularFrustumMesh,
         planar_projection: Option<&setup::PlanarTextureProjection>|
         -> TriangleMeshID {
            setup_triangle_mesh_for_new_entity(
                &mut resource_manager,
                TriangleMeshTemplate::CircularFrustum(*circular_frustum_mesh),
                planar_projection,
            )
        },
        ![TriangleMeshID]
    );

    setup!(
        {
            let mut resource_manager = resource_manager.owrite();
        },
        components,
        |sphere_mesh: &SphereMesh,
         planar_projection: Option<&setup::PlanarTextureProjection>|
         -> TriangleMeshID {
            setup_triangle_mesh_for_new_entity(
                &mut resource_manager,
                TriangleMeshTemplate::Sphere(*sphere_mesh),
                planar_projection,
            )
        },
        ![TriangleMeshID]
    );

    setup!(
        {
            let mut resource_manager = resource_manager.owrite();
        },
        components,
        |hemisphere_mesh: &HemisphereMesh,
         planar_projection: Option<&setup::PlanarTextureProjection>|
         -> TriangleMeshID {
            setup_triangle_mesh_for_new_entity(
                &mut resource_manager,
                TriangleMeshTemplate::Hemisphere(*hemisphere_mesh),
                planar_projection,
            )
        },
        ![TriangleMeshID]
    );

    setup!(
        {
            let mut resource_manager = resource_manager.owrite();
        },
        components,
        |capsule_mesh: &CapsuleMesh,
         planar_projection: Option<&setup::PlanarTextureProjection>|
         -> TriangleMeshID {
            setup_triangle_mesh_for_new_entity(
                &mut resource_manager,
                TriangleMeshTemplate::Capsule(*capsule_mesh),
                planar_projection,
            )
        },
        ![TriangleMeshID]
    );

    Ok(())
}

fn setup_triangle_mesh_for_new_entity(
    resource_manager: &mut ResourceManager,
    template: TriangleMeshTemplate,
    planar_projection: Option<&setup::PlanarTextureProjection>,
) -> TriangleMeshID {
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
            Option::<&PlanarTextureProjection>::None,
        ),
    }
}

/// Checks if the entities-to-be with the given components have a material
/// component and a component for a mesh that misses vertex attributes required
/// by the material, and if so, generates the missing vertex attributes if
/// possible.
pub fn generate_missing_vertex_properties_for_new_entity_meshes(
    resource_manager: &RwLock<ResourceManager>,
    components: &ArchetypeComponentStorage,
) {
    setup!(
        {
            let mut resource_manager = resource_manager.owrite();
        },
        components,
        |mesh_id: &TriangleMeshID, material_id: &MaterialID| {
            let resource_manager = &mut *resource_manager;
            let Some(material_template) =
                resource_manager
                    .materials
                    .get(*material_id)
                    .and_then(|material| {
                        resource_manager
                            .material_templates
                            .get(material.template_id)
                    })
            else {
                log::warn!(
                    "Tried to generate missing vertex properties for missing material {material_id}"
                );
                return;
            };

            let vertex_attribute_requirements = material_template.vertex_attribute_requirements;

            setup::generate_missing_vertex_properties_for_mesh(
                &mut resource_manager.triangle_meshes,
                *mesh_id,
                vertex_attribute_requirements,
            );
        }
    );
}
