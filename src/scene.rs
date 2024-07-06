//! Scene containing data to render.

mod camera;
mod events;
mod graph;
mod instance;
mod light;
mod mesh;
mod model;
mod postprocessing;
mod systems;
mod tasks;
mod texture_projection;
mod voxel;

pub use camera::{
    register_camera_components, OrthographicCameraComp, PerspectiveCameraComp, SceneCamera,
};
pub use events::RenderResourcesDesynchronized;
pub use graph::{
    register_scene_graph_components, CameraNodeID, GroupNodeID, ModelInstanceNodeID, NodeStorage,
    NodeTransform, ParentComp, SceneGraph, SceneGraphCameraNodeComp, SceneGraphGroupComp,
    SceneGraphGroupNodeComp, SceneGraphModelInstanceNodeComp, SceneGraphNodeComp, SceneGraphNodeID,
    SceneGraphParentNodeComp, UncullableComp, VoxelTreeNode, VoxelTreeNodeID,
};
pub use instance::InstanceFeatureManager;
pub use light::{
    compute_luminance_for_uniform_illuminance, register_light_components, AmbientEmissionComp,
    AmbientLight, AmbientLightComp, Illumninance, LightID, LightStorage, LightType, Luminance,
    LuminousIntensity, OmnidirectionalEmissionComp, OmnidirectionalLight, OmnidirectionalLightComp,
    UnidirectionalEmissionComp, UnidirectionalLight, UnidirectionalLightComp,
    MAX_SHADOW_MAP_CASCADES,
};
pub use mesh::{
    register_mesh_components, BoxMeshComp, CircularFrustumMeshComp, ConeMeshComp, CylinderMeshComp,
    HemisphereMeshComp, MeshComp, MeshID, MeshRepository, RectangleMeshComp, SphereMeshComp,
    SCREEN_FILLING_QUAD_MESH_ID,
};
pub use model::ModelID;
pub use postprocessing::{AmbientOcclusionConfig, BloomConfig, Postprocessor};
pub use systems::{SyncLightsInStorage, SyncSceneObjectTransforms};
pub use tasks::{
    BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances,
    BoundUnidirectionalLightsAndBufferShadowCastingModelInstances, BufferVisibleModelInstances,
    SyncSceneCameraViewTransform, UpdateSceneGroupToWorldTransforms,
};
pub use texture_projection::{register_texture_projection_components, PlanarTextureProjectionComp};
pub use voxel::{
    register_voxel_components, VoxelBoxComp, VoxelManager, VoxelSphereComp, VoxelTreeComp,
    VoxelTreeID, VoxelTreeNodeComp, VoxelTypeComp,
};

use crate::{
    assets::Assets,
    geometry::Radians,
    gpu::{rendering::fre, shader::ShaderManager, GraphicsDevice},
    material::{MaterialLibrary, ToneMapping},
};
use std::sync::RwLock;

/// Container for data needed to render a scene.
#[derive(Debug)]
pub struct Scene {
    config: SceneConfig,
    mesh_repository: RwLock<MeshRepository<fre>>,
    material_library: RwLock<MaterialLibrary>,
    light_storage: RwLock<LightStorage>,
    instance_feature_manager: RwLock<InstanceFeatureManager>,
    shader_manager: RwLock<ShaderManager>,
    voxel_manager: RwLock<VoxelManager<fre>>,
    scene_graph: RwLock<SceneGraph<fre>>,
    scene_camera: RwLock<Option<SceneCamera<fre>>>,
    postprocessor: RwLock<Postprocessor>,
}

/// Global scene configuration options.
#[derive(Clone, Debug)]
pub struct SceneConfig {
    pub voxel_extent: fre,
    pub initial_min_angular_voxel_extent_for_lod: Radians<fre>,
    pub ambient_occlusion: AmbientOcclusionConfig,
    pub bloom: BloomConfig,
    pub tone_mapping: ToneMapping,
    pub initial_exposure: fre,
}

impl Scene {
    /// Creates a new scene data container.
    pub fn new(config: SceneConfig, graphics_device: &GraphicsDevice, assets: &Assets) -> Self {
        let mut mesh_repository = MeshRepository::new();
        mesh_repository.create_default_meshes();

        let mut instance_feature_manager = InstanceFeatureManager::new();

        let mut material_library = MaterialLibrary::new();
        material_library.register_materials(&mut instance_feature_manager);

        let voxel_manager = VoxelManager::create(
            config.voxel_extent,
            config.initial_min_angular_voxel_extent_for_lod,
            graphics_device,
            assets,
            &mut mesh_repository,
            &mut material_library,
            &mut instance_feature_manager,
        );

        let postprocessor = Postprocessor::new(
            graphics_device,
            &mut material_library,
            &config.ambient_occlusion,
            &config.bloom,
            config.tone_mapping,
            config.initial_exposure,
        );

        Self {
            config,
            mesh_repository: RwLock::new(mesh_repository),
            material_library: RwLock::new(material_library),
            light_storage: RwLock::new(LightStorage::new()),
            instance_feature_manager: RwLock::new(instance_feature_manager),
            shader_manager: RwLock::new(ShaderManager::new()),
            voxel_manager: RwLock::new(voxel_manager),
            scene_graph: RwLock::new(SceneGraph::new()),
            scene_camera: RwLock::new(None),
            postprocessor: RwLock::new(postprocessor),
        }
    }

    /// Returns a reference to the global scene configuration.
    pub fn config(&self) -> &SceneConfig {
        &self.config
    }

    /// Returns a reference to the [`MeshRepository`], guarded by a [`RwLock`].
    pub fn mesh_repository(&self) -> &RwLock<MeshRepository<fre>> {
        &self.mesh_repository
    }

    /// Returns a reference to the [`MaterialLibrary`], guarded by a [`RwLock`].
    pub fn material_library(&self) -> &RwLock<MaterialLibrary> {
        &self.material_library
    }

    /// Returns a reference to the [`LightStorage`], guarded by a [`RwLock`].
    pub fn light_storage(&self) -> &RwLock<LightStorage> {
        &self.light_storage
    }

    /// Returns a reference to the [`InstanceFeatureManager`], guarded by a
    /// [`RwLock`].
    pub fn instance_feature_manager(&self) -> &RwLock<InstanceFeatureManager> {
        &self.instance_feature_manager
    }

    /// Returns a reference to the [`ShaderManager`], guarded by a [`RwLock`].
    pub fn shader_manager(&self) -> &RwLock<ShaderManager> {
        &self.shader_manager
    }

    /// Returns a reference to the [`VoxelManager`], guarded by a [`RwLock`].
    pub fn voxel_manager(&self) -> &RwLock<VoxelManager<fre>> {
        &self.voxel_manager
    }

    /// Returns a reference to the [`SceneGraph`], guarded by a [`RwLock`].
    pub fn scene_graph(&self) -> &RwLock<SceneGraph<fre>> {
        &self.scene_graph
    }

    /// Returns a reference to the [`SceneCamera`], or [`None`] if no scene
    /// camera has been set, guarded by a [`RwLock`].
    pub fn scene_camera(&self) -> &RwLock<Option<SceneCamera<fre>>> {
        &self.scene_camera
    }

    /// Returns a reference to the [`Postprocessor`], guarded by a [`RwLock`].
    pub fn postprocessor(&self) -> &RwLock<Postprocessor> {
        &self.postprocessor
    }

    /// Toggles ambient occlusion.
    pub fn toggle_ambient_occlusion(&self) {
        self.postprocessor
            .write()
            .unwrap()
            .toggle_ambient_occlusion();
    }

    /// Toggles bloom.
    pub fn toggle_bloom(&self) {
        self.postprocessor.write().unwrap().toggle_bloom();
    }

    /// Cycle tone mapping.
    pub fn cycle_tone_mapping(&self) {
        self.postprocessor.write().unwrap().cycle_tone_mapping();
    }
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self {
            voxel_extent: 0.25,
            initial_min_angular_voxel_extent_for_lod: Radians(0.0),
            ambient_occlusion: AmbientOcclusionConfig::default(),
            bloom: BloomConfig::default(),
            tone_mapping: ToneMapping::default(),
            initial_exposure: 1e-4,
        }
    }
}
