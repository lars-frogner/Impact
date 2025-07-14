//! Management of rigid bodies for entities.

use anyhow::{Result, anyhow};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_geometry::ReferenceFrame;
use impact_mesh::{
    MeshRepository, TriangleMeshID,
    setup::{BoxMesh, ConeMesh, CylinderMesh, HemisphereMesh, SphereMesh},
};
use impact_physics::{
    fph,
    quantities::Motion,
    rigid_body::{
        self, DynamicRigidBodyID, KinematicRigidBodyID, RigidBodyManager,
        setup::DynamicRigidBodySubstance,
    },
};
use std::sync::RwLock;

/// Checks if the entity-to-be with the given components has the components
/// representing a dynamic or kinematic rigid body, and if so, creates the
/// corresponding rigid body and adds a [`DynamicRigidBodyID`] or
/// [`KinematicRigidBodyID`] to the entity.
pub fn setup_rigid_body_for_new_entity(
    rigid_body_manager: &RwLock<RigidBodyManager>,
    mesh_repository: &RwLock<MeshRepository>,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    setup!(
        {
            let mut rigid_body_manager = rigid_body_manager.write().unwrap();
        },
        components,
        |box_mesh: &BoxMesh,
         substance: &DynamicRigidBodySubstance,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>|
         -> (DynamicRigidBodyID, ReferenceFrame, Motion) {
            rigid_body::setup::setup_dynamic_rigid_body_for_uniform_box(
                &mut rigid_body_manager,
                fph::from(box_mesh.extent_x),
                fph::from(box_mesh.extent_y),
                fph::from(box_mesh.extent_z),
                substance,
                frame.copied().unwrap_or_default(),
                motion.copied().unwrap_or_default(),
            )
        },
        ![DynamicRigidBodyID]
    );

    setup!(
        {
            let mut rigid_body_manager = rigid_body_manager.write().unwrap();
        },
        components,
        |cylinder_mesh: &CylinderMesh,
         substance: &DynamicRigidBodySubstance,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>|
         -> (DynamicRigidBodyID, ReferenceFrame, Motion) {
            rigid_body::setup::setup_dynamic_rigid_body_for_uniform_cylinder(
                &mut rigid_body_manager,
                fph::from(cylinder_mesh.length),
                fph::from(cylinder_mesh.diameter),
                substance,
                frame.copied().unwrap_or_default(),
                motion.copied().unwrap_or_default(),
            )
        },
        ![DynamicRigidBodyID]
    );

    setup!(
        {
            let mut rigid_body_manager = rigid_body_manager.write().unwrap();
        },
        components,
        |cone_mesh: &ConeMesh,
         substance: &DynamicRigidBodySubstance,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>|
         -> (DynamicRigidBodyID, ReferenceFrame, Motion) {
            rigid_body::setup::setup_dynamic_rigid_body_for_uniform_cone(
                &mut rigid_body_manager,
                fph::from(cone_mesh.length),
                fph::from(cone_mesh.max_diameter),
                substance,
                frame.copied().unwrap_or_default(),
                motion.copied().unwrap_or_default(),
            )
        },
        ![DynamicRigidBodyID]
    );

    setup!(
        {
            let mut rigid_body_manager = rigid_body_manager.write().unwrap();
        },
        components,
        |substance: &DynamicRigidBodySubstance,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>|
         -> (DynamicRigidBodyID, ReferenceFrame, Motion) {
            rigid_body::setup::setup_dynamic_rigid_body_for_uniform_sphere(
                &mut rigid_body_manager,
                substance,
                frame.copied().unwrap_or_default(),
                motion.copied().unwrap_or_default(),
            )
        },
        [SphereMesh],
        ![DynamicRigidBodyID]
    );

    setup!(
        {
            let mut rigid_body_manager = rigid_body_manager.write().unwrap();
        },
        components,
        |substance: &DynamicRigidBodySubstance,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>|
         -> (DynamicRigidBodyID, ReferenceFrame, Motion) {
            rigid_body::setup::setup_dynamic_rigid_body_for_uniform_hemisphere(
                &mut rigid_body_manager,
                substance,
                frame.copied().unwrap_or_default(),
                motion.copied().unwrap_or_default(),
            )
        },
        [HemisphereMesh],
        ![DynamicRigidBodyID]
    );

    setup!(
        {
            let mut rigid_body_manager = rigid_body_manager.write().unwrap();
            let mesh_repository = mesh_repository.read().unwrap();
        },
        components,
        |mesh_id: &TriangleMeshID,
         substance: &DynamicRigidBodySubstance,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>|
         -> Result<(DynamicRigidBodyID, ReferenceFrame, Motion)> {
            let triangle_mesh = mesh_repository.get_triangle_mesh(*mesh_id).ok_or_else(|| {
                anyhow!(
                    "Tried to create rigid body for missing mesh (mesh ID {})",
                    mesh_id
                )
            })?;
            rigid_body::setup::setup_dynamic_rigid_body_for_uniform_triangle_mesh(
                &mut rigid_body_manager,
                triangle_mesh.triangle_vertex_positions(),
                substance,
                frame.copied().unwrap_or_default(),
                motion.copied().unwrap_or_default(),
            )
        },
        ![DynamicRigidBodyID]
    )?;

    setup!(
        {
            let mut rigid_body_manager = rigid_body_manager.write().unwrap();
        },
        components,
        |frame: Option<&ReferenceFrame>, motion: &Motion| -> KinematicRigidBodyID {
            rigid_body::setup::setup_kinematic_rigid_body(
                &mut rigid_body_manager,
                frame.copied().unwrap_or_default(),
                *motion,
            )
        },
        ![DynamicRigidBodyID, KinematicRigidBodyID]
    );

    Ok(())
}
