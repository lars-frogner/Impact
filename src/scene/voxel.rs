//! Management of voxels.

mod components;

pub use components::{
    register_voxel_components, VoxelBoxComp, VoxelSphereComp, VoxelTreeComp, VoxelTreeNodeComp,
    VoxelTypeComp,
};

use crate::{
    assets::Assets,
    geometry::{
        FrontFaceSide, InstanceFeatureID, Radians, TriangleMesh, UniformBoxVoxelGenerator,
        UniformSphereVoxelGenerator, VoxelPropertyMap, VoxelTree, VoxelTreeLODController,
        VoxelType,
    },
    gpu::{
        rendering::fre,
        shader::{DiffuseMicrofacetShadingModel, SpecularMicrofacetShadingModel},
        GraphicsDevice,
    },
    num::Float,
    scene::{
        material::setup_microfacet_material, AlbedoComp, InstanceFeatureManager, MaterialComp,
        MaterialHandle, MaterialLibrary, MeshID, MeshRepository, ModelID, RGBColor, RoughnessComp,
        SpecularReflectanceComp,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_utils::hash64;
use lazy_static::lazy_static;
use nalgebra::vector;
use std::{collections::HashMap, sync::RwLock};

/// Descriptor for the appearance of a voxel type.
#[derive(Clone, Debug)]
pub struct VoxelAppearance {
    /// The ID of the voxel model.
    pub model_id: ModelID,
    /// The handle for the voxel's material.
    pub material_handle: MaterialHandle,
    /// The handle for the voxel's prepass material, if applicable.
    pub prepass_material_handle: Option<MaterialHandle>,
}

/// Identifier for a [`VoxelTree`] in a [`VoxelManager`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct VoxelTreeID(u32);

/// Manager of all [`VoxelTree`]s in a scene.
#[derive(Debug)]
pub struct VoxelManager<F: Float> {
    voxel_appearances: VoxelPropertyMap<VoxelAppearance>,
    voxel_material_feature_ids: VoxelPropertyMap<InstanceFeatureID>,
    voxel_tree_lod_controller: VoxelTreeLODController<F>,
    voxel_trees: HashMap<VoxelTreeID, VoxelTree<F>>,
    voxel_tree_id_counter: u32,
}

lazy_static! {
    /// The ID of the [`TriangleMesh`] in the [`MeshRepository`] representing a
    /// standard voxel.
    pub static ref VOXEL_MESH_ID: MeshID = MeshID(hash64!("VoxelMesh"));
}

#[cfg(test)]
impl VoxelTreeID {
    /// Creates a dummy [`VoxelTreeID`] that will never match an actual ID
    /// returned from the [`VoxelManager`]. Used for testing purposes.
    pub fn dummy() -> Self {
        Self(0)
    }
}

impl std::fmt::Display for VoxelTreeID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<F: Float> VoxelManager<F> {
    /// Returns a reference to the map from voxel types to material property
    /// feature IDs.
    pub fn voxel_material_feature_ids(&self) -> &VoxelPropertyMap<InstanceFeatureID> {
        &self.voxel_material_feature_ids
    }

    /// Returns a reference to the [`VoxelAppearance`] for the given voxel type.
    pub fn voxel_appearance(&self, voxel_type: VoxelType) -> &VoxelAppearance {
        self.voxel_appearances.value(voxel_type)
    }

    /// Returns a reference to the voxel tree LOD controller.
    pub fn voxel_tree_lod_controller(&self) -> &VoxelTreeLODController<F> {
        &self.voxel_tree_lod_controller
    }

    /// Returns a reference to the [`VoxelTree`] with the given ID, or [`None`]
    /// if the voxel tree is not present.
    pub fn get_voxel_tree(&self, voxel_tree_id: VoxelTreeID) -> Option<&VoxelTree<F>> {
        self.voxel_trees.get(&voxel_tree_id)
    }

    /// Scales the minimum angular voxel extent in the voxel tree LOD controller
    /// by the given factor. The extent should be scaled to remain proportional
    /// to the field of view and inversely proportional to the number of pixels
    /// across the window.
    pub fn scale_min_angular_voxel_extent_for_lod(&mut self, scale: F) {
        self.voxel_tree_lod_controller
            .scale_min_angular_voxel_extent(scale);
    }

    /// Returns a mutable reference to the [`VoxelTree`] with the given ID, or
    /// [`None`] if the voxel tree is not present.
    pub fn get_voxel_tree_mut(&mut self, voxel_tree_id: VoxelTreeID) -> Option<&mut VoxelTree<F>> {
        self.voxel_trees.get_mut(&voxel_tree_id)
    }

    /// Whether a voxel tree with the given ID exists in the manager.
    pub fn has_voxel_tree(&self, voxel_tree_id: VoxelTreeID) -> bool {
        self.voxel_trees.contains_key(&voxel_tree_id)
    }

    /// Returns a reference to the [`HashMap`] storing all voxel trees.
    pub fn voxel_trees(&self) -> &HashMap<VoxelTreeID, VoxelTree<F>> {
        &self.voxel_trees
    }

    /// Adds the given [`VoxelTree`] to the manager.
    ///
    /// # Returns
    /// A new [`VoxelTreeID`] representing the added voxel tree.
    pub fn add_voxel_tree(&mut self, voxel_tree: VoxelTree<F>) -> VoxelTreeID {
        let voxel_tree_id = self.create_new_voxel_tree_id();
        self.voxel_trees.insert(voxel_tree_id, voxel_tree);
        voxel_tree_id
    }

    fn create_new_voxel_tree_id(&mut self) -> VoxelTreeID {
        let voxel_tree_id = VoxelTreeID(self.voxel_tree_id_counter);
        self.voxel_tree_id_counter = self.voxel_tree_id_counter.checked_add(1).unwrap();
        voxel_tree_id
    }
}

impl VoxelManager<fre> {
    pub fn create(
        voxel_extent: fre,
        initial_min_angular_voxel_extent_for_lod: Radians<fre>,
        graphics_device: &GraphicsDevice,
        assets: &Assets,
        mesh_repository: &mut MeshRepository<fre>,
        material_library: &mut MaterialLibrary,
        instance_feature_manager: &mut InstanceFeatureManager,
    ) -> Self {
        mesh_repository.add_mesh_unless_present(
            *VOXEL_MESH_ID,
            TriangleMesh::create_box(
                voxel_extent,
                voxel_extent,
                voxel_extent,
                FrontFaceSide::Outside,
            ),
        );

        let voxel_appearances = VoxelType::all().map(|voxel_type| {
            let material = setup_voxel_material(
                voxel_type,
                graphics_device,
                assets,
                material_library,
                instance_feature_manager,
            );

            let material_handle = *material.material_handle();
            let prepass_material_handle = material.prepass_material_handle().cloned();

            let model_id = ModelID::for_mesh_and_material(
                *VOXEL_MESH_ID,
                material_handle,
                prepass_material_handle,
            );

            instance_feature_manager.register_instance(material_library, model_id);

            VoxelAppearance {
                model_id,
                material_handle,
                prepass_material_handle,
            }
        });

        let voxel_material_feature_ids = voxel_appearances
            .iter()
            .map(|appearance| {
                appearance
                    .material_handle
                    .material_property_feature_id()
                    .unwrap()
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        Self {
            voxel_appearances: VoxelPropertyMap::new(voxel_appearances),
            voxel_material_feature_ids: VoxelPropertyMap::new(voxel_material_feature_ids),
            voxel_tree_lod_controller: VoxelTreeLODController::new(
                initial_min_angular_voxel_extent_for_lod,
            ),
            voxel_trees: HashMap::new(),
            voxel_tree_id_counter: 1,
        }
    }

    pub fn add_voxel_tree_component_for_entity(
        voxel_manager: &RwLock<VoxelManager<fre>>,
        components: &mut ArchetypeComponentStorage,
        voxel_extent: fre,
    ) {
        setup!(
            {
                let mut voxel_manager = voxel_manager.write().unwrap();
            },
            components,
            |voxel_box: &VoxelBoxComp, voxel_type: &VoxelTypeComp| -> VoxelTreeComp {
                let generator = UniformBoxVoxelGenerator::new(
                    voxel_type.voxel_type(),
                    voxel_extent,
                    voxel_box.size_x,
                    voxel_box.size_y,
                    voxel_box.size_z,
                );

                let voxel_tree =
                    VoxelTree::build(&generator).expect("Tried to build tree for empty voxel box");

                let voxel_tree_id = voxel_manager.add_voxel_tree(voxel_tree);

                VoxelTreeComp { voxel_tree_id }
            },
            ![VoxelTreeComp]
        );

        setup!(
            {
                let mut voxel_manager = voxel_manager.write().unwrap();
            },
            components,
            |voxel_sphere: &VoxelSphereComp, voxel_type: &VoxelTypeComp| -> VoxelTreeComp {
                let generator = UniformSphereVoxelGenerator::new(
                    voxel_type.voxel_type(),
                    voxel_extent,
                    voxel_sphere.n_voxels_across(),
                    voxel_sphere.instance_group_height(),
                );

                let voxel_tree = VoxelTree::build(&generator)
                    .expect("Tried to build tree for empty voxel sphere");

                let voxel_tree_id = voxel_manager.add_voxel_tree(voxel_tree);

                VoxelTreeComp { voxel_tree_id }
            },
            ![VoxelTreeComp]
        );
    }
}

fn setup_voxel_material(
    voxel_type: VoxelType,
    graphics_device: &GraphicsDevice,
    assets: &Assets,
    material_library: &mut MaterialLibrary,
    instance_feature_manager: &mut InstanceFeatureManager,
) -> MaterialComp {
    match voxel_type {
        VoxelType::Default => setup_microfacet_material_for_voxel(
            graphics_device,
            assets,
            material_library,
            instance_feature_manager,
            vector![0.5, 0.5, 0.5],
            Some(SpecularReflectanceComp::in_range_of(
                SpecularReflectanceComp::STONE,
                0.5,
            )),
            Some(0.7),
        ),
    }
}

fn setup_microfacet_material_for_voxel(
    graphics_device: &GraphicsDevice,
    assets: &Assets,
    material_library: &mut MaterialLibrary,
    instance_feature_manager: &mut InstanceFeatureManager,
    albedo: RGBColor,
    specular_reflectance: Option<SpecularReflectanceComp>,
    roughness: Option<f32>,
) -> MaterialComp {
    let roughness = roughness.map(RoughnessComp);

    let specular_shading_model = if specular_reflectance.is_some() {
        SpecularMicrofacetShadingModel::GGX
    } else {
        SpecularMicrofacetShadingModel::None
    };

    setup_microfacet_material(
        graphics_device,
        assets,
        material_library,
        instance_feature_manager,
        Some(&AlbedoComp(albedo)),
        specular_reflectance.as_ref(),
        None,
        None,
        None,
        roughness.as_ref(),
        None,
        None,
        None,
        DiffuseMicrofacetShadingModel::GGX,
        specular_shading_model,
    )
}
