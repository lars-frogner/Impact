//! ECS systems for driving voxel object interaction.

use crate::{
    HasVoxelObject, VoxelManager, VoxelObjectManager,
    collidable::{CollisionWorld, LocalCollidable, setup::VoxelCollidable},
    interaction::{
        self, VoxelAbsorbingCapsuleEntity, VoxelAbsorbingSphereEntity,
        VoxelObjectInteractionContext,
        absorption::{self, HasVoxelAbsorbingCapsule, HasVoxelAbsorbingSphere},
    },
    voxel_types::VoxelTypeRegistry,
};
use impact_ecs::{
    component::{ComponentArray, ComponentFlags, ComponentStorage},
    metadata::ComponentMetadataRegistry,
    query,
    world::{EntityStager, World as ECSWorld},
};
use impact_geometry::{ModelTransform, ReferenceFrame};
use impact_id::{EntityID, EntityIDManager};
use impact_intersection::{IntersectionManager, bounding_volume::BoundingVolumeManager};
use impact_model::HasModel;
use impact_physics::{
    anchor::AnchorManager,
    collision::{CollidableID, HasCollidable},
    force::{
        ForceGeneratorManager,
        constant_acceleration::{
            ConstantAccelerationGeneratorID, HasConstantAccelerationGenerator,
        },
    },
    rigid_body::{HasDynamicRigidBody, RigidBodyManager},
};
use impact_scene::{
    ParentEntity, SceneEntityFlags,
    graph::{SceneGraph, SceneGroupID},
};
use tinyvec::TinyVec;

/// ECS-based implementation of a voxel object interaction context.
#[derive(Debug)]
pub struct ECSVoxelObjectInteractionContext<'a> {
    pub component_metadata_registry: &'a ComponentMetadataRegistry,
    pub entity_stager: &'a mut EntityStager,
    pub ecs_world: &'a ECSWorld,
    pub scene_graph: &'a SceneGraph,
    pub force_generator_manager: &'a ForceGeneratorManager,
    pub collision_world: &'a CollisionWorld,
}

impl<'a> VoxelObjectInteractionContext for ECSVoxelObjectInteractionContext<'a> {
    fn gather_voxel_absorbing_sphere_entities(
        &mut self,
    ) -> TinyVec<[VoxelAbsorbingSphereEntity; 4]> {
        let mut entities = TinyVec::new();

        query!(
            self.ecs_world,
            |entity_id: EntityID, reference_frame: &ReferenceFrame, flags: &SceneEntityFlags| {
                let sphere_to_world_transform = if !flags.is_disabled() {
                    Some(reference_frame.create_transform_to_parent_space())
                } else {
                    None
                };
                entities.push(VoxelAbsorbingSphereEntity {
                    entity_id,
                    sphere_to_world_transform,
                });
            },
            [HasVoxelAbsorbingSphere],
            ![ParentEntity]
        );

        query!(
            self.ecs_world,
            |entity_id: EntityID,
             reference_frame: &ReferenceFrame,
             parent: &ParentEntity,
             flags: &SceneEntityFlags| {
                let sphere_to_world_transform = if !flags.is_disabled() {
                    let parent_node = self
                        .scene_graph
                        .group_nodes()
                        .node(SceneGroupID::from_entity_id(parent.0));

                    let group_to_root_transform = parent_node.group_to_root_transform().aligned();

                    Some(
                        group_to_root_transform
                            * reference_frame.create_transform_to_parent_space(),
                    )
                } else {
                    None
                };
                entities.push(VoxelAbsorbingSphereEntity {
                    entity_id,
                    sphere_to_world_transform,
                });
            },
            [HasVoxelAbsorbingSphere]
        );

        entities
    }

    fn gather_voxel_absorbing_capsule_entities(
        &mut self,
    ) -> TinyVec<[VoxelAbsorbingCapsuleEntity; 4]> {
        let mut entities = TinyVec::new();

        query!(
            self.ecs_world,
            |entity_id: EntityID, reference_frame: &ReferenceFrame, flags: &SceneEntityFlags| {
                let capsule_to_world_transform = if !flags.is_disabled() {
                    Some(reference_frame.create_transform_to_parent_space())
                } else {
                    None
                };
                entities.push(VoxelAbsorbingCapsuleEntity {
                    entity_id,
                    capsule_to_world_transform,
                });
            },
            [HasVoxelAbsorbingCapsule],
            ![ParentEntity]
        );

        query!(
            self.ecs_world,
            |entity_id: EntityID,
             reference_frame: &ReferenceFrame,
             parent: &ParentEntity,
             flags: &SceneEntityFlags| {
                let capsule_to_world_transform = if !flags.is_disabled() {
                    let parent_node = self
                        .scene_graph
                        .group_nodes()
                        .node(SceneGroupID::from_entity_id(parent.0));

                    let group_to_root_transform = parent_node.group_to_root_transform().aligned();

                    Some(
                        group_to_root_transform
                            * reference_frame.create_transform_to_parent_space(),
                    )
                } else {
                    None
                };
                entities.push(VoxelAbsorbingCapsuleEntity {
                    entity_id,
                    capsule_to_world_transform,
                });
            },
            [HasVoxelAbsorbingCapsule]
        );

        entities
    }

    fn on_new_disconnected_voxel_object_entity(
        &mut self,
        new_entity_id: EntityID,
        parent_entity_id: EntityID,
    ) {
        let parent_components = self.ecs_world.entity(parent_entity_id).cloned_components();

        let mut components = Vec::with_capacity(parent_components.n_component_types());

        components.push(ComponentStorage::from_single_instance_view(&HasVoxelObject));
        components.push(ComponentStorage::from_single_instance_view(
            &HasDynamicRigidBody,
        ));

        if parent_components
            .archetype()
            .contains_component::<HasCollidable>()
        {
            let parent_collidable_id = CollidableID::from_entity_id(parent_entity_id);

            if let Some(descriptor) = self
                .collision_world
                .get_collidable_descriptor(parent_collidable_id)
                && let LocalCollidable::VoxelObject(local_collidable) =
                    descriptor.local_collidable()
            {
                components.push(ComponentStorage::from_single_instance_view(
                    &VoxelCollidable::new(descriptor.kind(), *local_collidable.response_params()),
                ));
            }
        }

        if parent_components
            .archetype()
            .contains_component::<HasConstantAccelerationGenerator>()
        {
            let parent_force_generator_id =
                ConstantAccelerationGeneratorID::from_entity_id(parent_entity_id);

            if let Some(force_generator) = self
                .force_generator_manager
                .constant_accelerations()
                .get_generator(&parent_force_generator_id)
            {
                components.push(ComponentStorage::from_single_instance_view(
                    &force_generator.acceleration,
                ));
            }
        }

        // TODO: We don't handle drag force yet (that would also have to be
        // updated for the original object, since its shape has changed)

        // Add directly inherited components
        for component_storage in parent_components.into_component_arrays() {
            let metadata = self
                .component_metadata_registry
                .metadata(component_storage.component_id());

            if metadata.flags.contains(ComponentFlags::INHERITABLE) {
                components.push(component_storage);
            }
        }

        self.entity_stager
            .stage_entity_for_creation_with_id(new_entity_id, components)
            .expect("Failed to stage voxel object entity for creation");
    }

    fn on_empty_voxel_object_entity(&mut self, entity_id: EntityID) {
        self.entity_stager.stage_entity_for_removal(entity_id);
    }
}

/// Synchronizes model transforms for all voxel objects entities with their
/// inertial properties.
pub fn sync_voxel_object_model_transforms(
    ecs_world: &ECSWorld,
    voxel_object_manager: &VoxelObjectManager,
) {
    query!(
        ecs_world,
        |entity_id: EntityID, model_transform: &mut ModelTransform| {
            interaction::sync_voxel_object_model_transform_with_inertial_properties(
                voxel_object_manager,
                entity_id,
                model_transform,
            );
        }
    );
}

/// Updates the bounding volumes of all voxel object's bounding volumes in the
/// bounding volume manager to match the current bounding sphere of the object.
pub fn sync_voxel_object_bounding_volumes(
    ecs_world: &ECSWorld,
    voxel_object_manager: &VoxelObjectManager,
    bounding_volume_manager: &mut BoundingVolumeManager,
) {
    query!(
        ecs_world,
        |entity_id: EntityID| {
            interaction::sync_voxel_object_bounding_volume(
                voxel_object_manager,
                bounding_volume_manager,
                entity_id,
            );
        },
        [HasVoxelObject, HasModel]
    );
}

/// Applies each voxel-absorbing sphere and capsule to the affected voxel
/// objects.
pub fn apply_absorption(
    component_metadata_registry: &ComponentMetadataRegistry,
    entity_id_manager: &mut EntityIDManager,
    entity_stager: &mut EntityStager,
    ecs_world: &ECSWorld,
    scene_graph: &SceneGraph,
    voxel_manager: &mut VoxelManager,
    voxel_type_registry: &VoxelTypeRegistry,
    intersection_manager: &IntersectionManager,
    rigid_body_manager: &mut RigidBodyManager,
    anchor_manager: &mut AnchorManager,
    force_generator_manager: &ForceGeneratorManager,
    collision_world: &CollisionWorld,
) {
    let mut interaction_context = ECSVoxelObjectInteractionContext {
        component_metadata_registry,
        entity_stager,
        ecs_world,
        scene_graph,
        force_generator_manager,
        collision_world,
    };

    absorption::apply_absorption(
        &mut interaction_context,
        entity_id_manager,
        voxel_manager,
        voxel_type_registry,
        intersection_manager,
        rigid_body_manager,
        anchor_manager,
    );
}
