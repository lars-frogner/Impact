//! ECS systems for driving voxel object interaction.

use crate::{
    HasVoxelObject, VoxelManager, VoxelObjectManager,
    collidable::{CollisionWorld, LocalCollidable, setup::VoxelCollidable},
    interaction::{
        self, NewVoxelObjectEntity, VoxelAbsorbingCapsuleEntity, VoxelAbsorbingSphereEntity,
        VoxelObjectEntity, VoxelObjectInteractionContext,
        absorption::{self, HasVoxelAbsorbingCapsule, HasVoxelAbsorbingSphere},
    },
    voxel_types::VoxelTypeRegistry,
};
use impact_alloc::{AVec, Allocator};
use impact_ecs::{
    component::{ComponentArray, ComponentFlags, ComponentStorage},
    metadata::ComponentMetadataRegistry,
    query,
    world::{EntityStager, World as ECSWorld},
};
use impact_geometry::{ModelTransform, ReferenceFrame};
use impact_id::{EntityID, EntityIDManager};
use impact_model::HasModel;
use impact_physics::{
    anchor::AnchorManager,
    collision::CollidableID,
    force::{ForceGeneratorManager, constant_acceleration::ConstantAccelerationGeneratorID},
    rigid_body::{DynamicRigidBodyID, RigidBodyManager},
};
use impact_scene::{
    ParentEntity, SceneEntityFlags,
    graph::{SceneGraph, SceneGroupID},
    setup::Uncullable,
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
    fn gather_voxel_object_entities<A: Allocator>(
        &mut self,
        entities: &mut AVec<VoxelObjectEntity, A>,
    ) {
        query!(
            self.ecs_world,
            |entity_id: EntityID, flags: &SceneEntityFlags| {
                if flags.is_disabled() {
                    return;
                }
                entities.push(VoxelObjectEntity { entity_id });
            },
            [HasVoxelObject, DynamicRigidBodyID] // We only let dynamic voxel objects participate in interactions
        );
    }

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
        entity: NewVoxelObjectEntity,
        parent_entity_id: EntityID,
    ) {
        let parent_components = self.ecs_world.entity(parent_entity_id).cloned_components();

        let mut components = Vec::with_capacity(parent_components.n_component_types());

        components.push(ComponentStorage::from_single_instance_view(
            &entity.rigid_body_id,
        ));

        if let Some(collidable_id) = parent_components.get_component::<CollidableID>()
            && let Some(descriptor) = self
                .collision_world
                .get_collidable_descriptor(*collidable_id)
            && let LocalCollidable::VoxelObject(local_collidable) = descriptor.local_collidable()
        {
            components.push(ComponentStorage::from_single_instance_view(
                &VoxelCollidable::new(descriptor.kind(), *local_collidable.response_params()),
            ));
        }

        if let Some(force_generator_id) =
            parent_components.get_component::<ConstantAccelerationGeneratorID>()
            && let Some(force_generator) = self
                .force_generator_manager
                .constant_accelerations()
                .get_generator(force_generator_id)
        {
            components.push(ComponentStorage::from_single_instance_view(
                &force_generator.acceleration,
            ));
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
            .stage_entity_for_creation_with_id(entity.entity_id, components)
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

/// Updates the bounding spheres of all voxel object's model instance nodes to match
/// the current bounding sphere of the object.
pub fn sync_voxel_object_bounding_spheres_in_scene_graph(
    ecs_world: &ECSWorld,
    voxel_object_manager: &VoxelObjectManager,
    scene_graph: &mut SceneGraph,
) {
    query!(
        ecs_world,
        |entity_id: EntityID| {
            interaction::sync_voxel_object_bounding_sphere_in_scene_graph(
                voxel_object_manager,
                scene_graph,
                entity_id,
                false,
            );
        },
        [HasVoxelObject, HasModel],
        ![Uncullable]
    );
    query!(
        ecs_world,
        |entity_id: EntityID| {
            interaction::sync_voxel_object_bounding_sphere_in_scene_graph(
                voxel_object_manager,
                scene_graph,
                entity_id,
                true,
            );
        },
        [HasVoxelObject, HasModel, Uncullable]
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
        rigid_body_manager,
        anchor_manager,
    );
}
