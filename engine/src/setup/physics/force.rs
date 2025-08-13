//! Setup of forces for new entities.

use crate::resource::ResourceManager;
use anyhow::{Result, anyhow};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_geometry::ModelTransform;
use impact_mesh::TriangleMeshID;
use impact_physics::{
    anchor::AnchorManager,
    force::{
        ForceGeneratorManager,
        constant_acceleration::ConstantAccelerationGeneratorID,
        detailed_drag::DetailedDragForceGeneratorID,
        local_force::LocalForceGeneratorID,
        setup::{self, ConstantAcceleration, DetailedDragProperties, LocalForce},
        spring_force::{
            DynamicDynamicSpringForceGeneratorID, DynamicDynamicSpringForceProperties,
            DynamicKinematicSpringForceGeneratorID, DynamicKinematicSpringForceProperties,
        },
    },
    rigid_body::DynamicRigidBodyID,
};
use parking_lot::RwLock;

pub fn setup_forces_for_new_entities(
    anchor_manager: &RwLock<AnchorManager>,
    force_generator_manager: &RwLock<ForceGeneratorManager>,
    resource_manager: &RwLock<ResourceManager>,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    setup!(
        {
            let mut force_generator_manager = force_generator_manager.write();
        },
        components,
        |rigid_body_id: &DynamicRigidBodyID,
         acceleration: &ConstantAcceleration|
         -> ConstantAccelerationGeneratorID {
            setup::setup_constant_acceleration(
                &mut force_generator_manager,
                *rigid_body_id,
                *acceleration,
            )
        }
    );

    setup!(
        {
            let mut anchor_manager = anchor_manager.write();
            let mut force_generator_manager = force_generator_manager.write();
        },
        components,
        |rigid_body_id: &DynamicRigidBodyID, local_force: &LocalForce| -> LocalForceGeneratorID {
            setup::setup_local_force(
                &mut anchor_manager,
                &mut force_generator_manager,
                *rigid_body_id,
                *local_force,
            )
        }
    );

    setup!(
        {
            let mut anchor_manager = anchor_manager.write();
            let mut force_generator_manager = force_generator_manager.write();
        },
        components,
        |properties: &DynamicDynamicSpringForceProperties| -> DynamicDynamicSpringForceGeneratorID {
            setup::setup_dynamic_dynamic_spring_force(
                &mut anchor_manager,
                &mut force_generator_manager,
                *properties,
            )
        }
    );

    setup!(
        {
            let mut anchor_manager = anchor_manager.write();
            let mut force_generator_manager = force_generator_manager.write();
        },
        components,
        |properties: &DynamicKinematicSpringForceProperties|
         -> DynamicKinematicSpringForceGeneratorID {
            setup::setup_dynamic_kinematic_spring_force(
                &mut anchor_manager,
                &mut force_generator_manager,
                *properties,
            )
        }
    );

    setup!(
        {
            let mut force_generator_manager = force_generator_manager.write();
            let resource_manager = resource_manager.read();
        },
        components,
        |drag_properties: &DetailedDragProperties,
         rigid_body_id: &DynamicRigidBodyID,
         model_transform: &ModelTransform,
         mesh_id: &TriangleMeshID|
         -> Result<DetailedDragForceGeneratorID> {
            let triangle_mesh =
                resource_manager
                    .triangle_meshes
                    .get(*mesh_id)
                    .ok_or_else(|| {
                        anyhow!("Tried to setup detailed drag for missing mesh {}", mesh_id)
                    })?;

            setup::setup_detailed_drag_force(
                &mut force_generator_manager,
                *drag_properties,
                *rigid_body_id,
                model_transform,
                (*mesh_id).into(),
                triangle_mesh.triangle_vertex_positions(),
            )
        }
    )?;

    Ok(())
}
