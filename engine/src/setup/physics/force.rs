//! Setup of forces for new entities.

use crate::resource::ResourceManager;
use anyhow::{Result, anyhow};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_geometry::ModelTransform;
use impact_math::hash64;
use impact_mesh::TriangleMeshHandle;
use impact_physics::{
    force::{
        ForceGeneratorManager,
        constant_acceleration::ConstantAccelerationGeneratorID,
        detailed_drag::DetailedDragForceGeneratorID,
        local_force::LocalForceGeneratorID,
        setup::{
            self, ConstantAcceleration, DetailedDragProperties, DynamicDynamicSpringForceGenerator,
            DynamicKinematicSpringForceGenerator, LocalForce,
        },
        spring_force::{
            DynamicDynamicSpringForceGeneratorID, DynamicKinematicSpringForceGeneratorID,
        },
    },
    rigid_body::DynamicRigidBodyID,
};
use parking_lot::RwLock;

pub fn setup_forces_for_new_entities(
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
            let mut force_generator_manager = force_generator_manager.write();
        },
        components,
        |rigid_body_id: &DynamicRigidBodyID, local_force: &LocalForce| -> LocalForceGeneratorID {
            setup::setup_local_force(&mut force_generator_manager, *rigid_body_id, *local_force)
        }
    );

    setup!(
        {
            let mut force_generator_manager = force_generator_manager.write();
        },
        components,
        |generator: &DynamicDynamicSpringForceGenerator| -> DynamicDynamicSpringForceGeneratorID {
            setup::setup_dynamic_dynamic_spring_force_generator(
                &mut force_generator_manager,
                *generator,
            )
        }
    );

    setup!(
        {
            let mut force_generator_manager = force_generator_manager.write();
        },
        components,
        |generator: &DynamicKinematicSpringForceGenerator|
         -> DynamicKinematicSpringForceGeneratorID {
            setup::setup_dynamic_kinematic_spring_force_generator(
                &mut force_generator_manager,
                *generator,
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
         mesh_handle: &TriangleMeshHandle|
         -> Result<DetailedDragForceGeneratorID> {
            let triangle_mesh = resource_manager
                .triangle_meshes
                .registry
                .get(*mesh_handle)
                .ok_or_else(|| {
                    anyhow!(
                        "Tried to setup detailed drag for missing mesh {}",
                        mesh_handle
                    )
                })?;

            // Only allow saving the map if the mesh is persistent
            let (drag_load_map_id, disable_saving) = resource_manager
                .triangle_meshes
                .index
                .get_pid(*mesh_handle)
                .map_or_else(
                    || (hash64!(mesh_handle.to_string()), true),
                    |mesh_id| (mesh_id.into(), false),
                );

            setup::setup_detailed_drag_force(
                &mut force_generator_manager,
                *drag_properties,
                *rigid_body_id,
                model_transform,
                drag_load_map_id,
                triangle_mesh.triangle_vertex_positions(),
                disable_saving,
            )
        }
    )?;

    Ok(())
}
