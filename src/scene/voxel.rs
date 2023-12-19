//! Management of voxels.

mod components;

pub use components::{
    register_voxel_components, VoxelBoxComp, VoxelInstanceClusterComp, VoxelSphereComp,
    VoxelTreeComp, VoxelTypeComp,
};

use crate::{
    geometry::{
        FrontFaceSide, TriangleMesh, UniformBoxVoxelGenerator, UniformSphereVoxelGenerator,
        VoxelTree, VoxelType,
    },
    num::Float,
    rendering::{fre, DiffuseMicrofacetShadingModel, SpecularMicrofacetShadingModel},
    scene::{
        material::setup_microfacet_material, DiffuseColorComp, InstanceFeatureManager,
        MaterialComp, MaterialHandle, MaterialLibrary, MeshID, MeshRepository, ModelID, RGBColor,
        RoughnessComp, SpecularColorComp,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_utils::hash64;
use lazy_static::lazy_static;
use nalgebra::vector;
use num_traits::ToPrimitive;
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
    voxel_appearances: Vec<VoxelAppearance>,
    voxel_trees: HashMap<VoxelTreeID, VoxelTree<F>>,
    voxel_tree_id_counter: u32,
}

lazy_static! {
    /// The ID of the [`TriangleMesh`] in the [`MeshRepository`] representing a
    /// standard voxel.
    pub static ref VOXEL_MESH_ID: MeshID = MeshID(hash64!("VoxelMesh"));
}

impl std::fmt::Display for VoxelTreeID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<F: Float> VoxelManager<F> {
    /// Returns a reference to the [`VoxelAppearance`] for the given voxel type.
    pub fn voxel_appearance(&self, voxel_type: VoxelType) -> &VoxelAppearance {
        &self.voxel_appearances[voxel_type.to_usize().unwrap()]
    }

    /// Returns a reference to the [`VoxelTree`] with the given ID, or [`None`]
    /// if the voxel tree is not present.
    pub fn get_voxel_tree(&self, voxel_tree_id: VoxelTreeID) -> Option<&VoxelTree<F>> {
        self.voxel_trees.get(&voxel_tree_id)
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

        let voxel_appearances = VoxelType::all()
            .map(|voxel_type| {
                let material =
                    setup_voxel_material(voxel_type, material_library, instance_feature_manager);

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
            })
            .collect();

        Self {
            voxel_appearances,
            voxel_trees: HashMap::new(),
            voxel_tree_id_counter: 0,
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
    material_library: &mut MaterialLibrary,
    instance_feature_manager: &mut InstanceFeatureManager,
) -> MaterialComp {
    match voxel_type {
        VoxelType::Default => setup_microfacet_material_for_voxel(
            material_library,
            instance_feature_manager,
            vector![0.5, 0.5, 0.5],
            Some(SpecularColorComp::in_range_of(
                SpecularColorComp::STONE,
                0.5,
            )),
            Some(0.7),
        ),
    }
}

fn setup_microfacet_material_for_voxel(
    material_library: &mut MaterialLibrary,
    instance_feature_manager: &mut InstanceFeatureManager,
    diffuse_color: RGBColor,
    specular_color: Option<SpecularColorComp>,
    roughness: Option<f32>,
) -> MaterialComp {
    let roughness = roughness.map(RoughnessComp);

    let specular_shading_model = if specular_color.is_some() {
        SpecularMicrofacetShadingModel::GGX
    } else {
        SpecularMicrofacetShadingModel::None
    };

    setup_microfacet_material(
        material_library,
        instance_feature_manager,
        Some(&DiffuseColorComp(diffuse_color)),
        specular_color.as_ref(),
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
