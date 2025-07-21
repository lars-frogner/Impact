//! Interaction in an ECS context.

use crate::{
    VoxelManager, VoxelObjectID, VoxelObjectManager,
    interaction::{
        self, NewVoxelObjectEntity, VoxelAbsorbingCapsuleEntity, VoxelAbsorbingSphereEntity,
        VoxelObjectEntity, VoxelObjectInteractionContext,
        absorption::{self, VoxelAbsorbingCapsule, VoxelAbsorbingSphere},
    },
};
use impact_ecs::{
    query,
    world::{EntityID, EntityStager, World as ECSWorld},
};
use impact_geometry::{ModelTransform, ReferenceFrame};
use impact_physics::{fph, rigid_body::RigidBodyManager};
use impact_scene::{SceneEntityFlags, SceneGraphParentNodeHandle, graph::SceneGraph};
use tinyvec::TinyVec;

/// ECS-based implementation of a voxel object interaction context.
#[derive(Debug)]
pub struct ECSVoxelObjectInteractionContext<'a> {
    pub entity_stager: &'a mut EntityStager,
    pub ecs_world: &'a ECSWorld,
    pub scene_graph: &'a SceneGraph,
}

impl<'a> VoxelObjectInteractionContext for ECSVoxelObjectInteractionContext<'a> {
    type EntityID = EntityID;

    fn gather_voxel_object_entities(&mut self, entities: &mut Vec<VoxelObjectEntity<EntityID>>) {
        query!(
            self.ecs_world,
            |entity_id: EntityID, voxel_object_id: &VoxelObjectID, flags: &SceneEntityFlags| {
                if flags.is_disabled() {
                    return;
                }
                entities.push(VoxelObjectEntity {
                    entity_id,
                    voxel_object_id: *voxel_object_id,
                });
            }
        );
    }

    fn gather_voxel_absorbing_sphere_entities(
        &mut self,
    ) -> TinyVec<[VoxelAbsorbingSphereEntity; 4]> {
        let mut entities = TinyVec::new();

        query!(
            self.ecs_world,
            |sphere: &VoxelAbsorbingSphere,
             reference_frame: &ReferenceFrame,
             flags: &SceneEntityFlags| {
                if flags.is_disabled() {
                    return;
                }
                entities.push(VoxelAbsorbingSphereEntity {
                    sphere: *sphere,
                    sphere_to_world_transform: reference_frame.create_transform_to_parent_space(),
                });
            },
            ![SceneGraphParentNodeHandle]
        );

        query!(
            self.ecs_world,
            |sphere: &VoxelAbsorbingSphere,
             reference_frame: &ReferenceFrame,
             parent: &SceneGraphParentNodeHandle,
             flags: &SceneEntityFlags| {
                if flags.is_disabled() {
                    return;
                }

                let parent_node = self.scene_graph.group_nodes().node(parent.id);

                let sphere_to_world_transform = parent_node.group_to_root_transform().cast()
                    * reference_frame.create_transform_to_parent_space::<f64>();

                entities.push(VoxelAbsorbingSphereEntity {
                    sphere: *sphere,
                    sphere_to_world_transform,
                });
            }
        );

        entities
    }

    fn gather_voxel_absorbing_capsule_entities(
        &mut self,
    ) -> TinyVec<[VoxelAbsorbingCapsuleEntity; 4]> {
        let mut entities = TinyVec::new();

        query!(
            self.ecs_world,
            |capsule: &VoxelAbsorbingCapsule,
             reference_frame: &ReferenceFrame,
             flags: &SceneEntityFlags| {
                if flags.is_disabled() {
                    return;
                }
                entities.push(VoxelAbsorbingCapsuleEntity {
                    capsule: *capsule,
                    capsule_to_world_transform: reference_frame.create_transform_to_parent_space(),
                });
            },
            ![SceneGraphParentNodeHandle]
        );

        query!(
            self.ecs_world,
            |capsule: &VoxelAbsorbingCapsule,
             reference_frame: &ReferenceFrame,
             parent: &SceneGraphParentNodeHandle,
             flags: &SceneEntityFlags| {
                if flags.is_disabled() {
                    return;
                }

                let parent_node = self.scene_graph.group_nodes().node(parent.id);

                let capsule_to_world_transform = parent_node.group_to_root_transform().cast()
                    * reference_frame.create_transform_to_parent_space::<f64>();

                entities.push(VoxelAbsorbingCapsuleEntity {
                    capsule: *capsule,
                    capsule_to_world_transform,
                });
            }
        );

        entities
    }

    fn on_new_voxel_object_entity(&mut self, entity: NewVoxelObjectEntity) {
        self.entity_stager
            .stage_entity_for_creation((&entity.voxel_object_id, &entity.rigid_body_id))
            .expect("Failed to stage voxel object entity for creation");
    }

    fn on_empty_voxel_object_entity(&mut self, entity_id: EntityID) {
        self.entity_stager.stage_entity_for_removal(entity_id);
    }
}

/// Synchronizes model transforms for all voxel objects entities with their
/// inertial properties.
pub fn sync_voxel_object_model_transforms(
    ecs_world: &mut ECSWorld,
    voxel_object_manager: &VoxelObjectManager,
) {
    query!(
        ecs_world,
        |voxel_object_id: &VoxelObjectID, model_transform: &mut ModelTransform| {
            interaction::sync_voxel_object_model_transform_with_inertial_properties(
                voxel_object_manager,
                *voxel_object_id,
                model_transform,
            );
        }
    );
}

/// Applies each voxel-absorbing sphere and capsule to the affected voxel
/// objects.
pub fn apply_absorption(
    entity_stager: &mut EntityStager,
    ecs_world: &ECSWorld,
    scene_graph: &SceneGraph,
    voxel_manager: &mut VoxelManager,
    rigid_body_manager: &mut RigidBodyManager,
    time_step_duration: fph,
) {
    let mut interaction_context = ECSVoxelObjectInteractionContext {
        entity_stager,
        ecs_world,
        scene_graph,
    };

    absorption::apply_absorption(
        &mut interaction_context,
        voxel_manager,
        rigid_body_manager,
        time_step_duration,
    );
}
