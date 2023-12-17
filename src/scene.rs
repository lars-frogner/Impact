//! Scene containing data to render.

mod camera;
mod events;
mod graph;
mod instance;
pub mod io;
mod light;
mod material;
mod mesh;
mod model;
mod shader;
mod systems;
mod tasks;
mod texture_projection;
mod voxel;

pub use camera::{
    register_camera_components, OrthographicCameraComp, PerspectiveCameraComp, SceneCamera,
};
pub use events::RenderResourcesDesynchronized;
pub use graph::{
    register_scene_graph_components, CameraNodeID, GroupNodeID, ModelInstanceClusterNode,
    ModelInstanceClusterNodeID, ModelInstanceNodeID, NodeStorage, NodeTransform, ParentComp,
    SceneGraph, SceneGraphCameraNodeComp, SceneGraphGroupComp, SceneGraphGroupNodeComp,
    SceneGraphModelInstanceNodeComp, SceneGraphNodeComp, SceneGraphNodeID,
    SceneGraphParentNodeComp, UncullableComp,
};
pub use instance::InstanceFeatureManager;
pub use light::{
    compute_radiance_for_uniform_irradiance, register_light_components, AmbientLight,
    AmbientLightComp, AngularExtentComp, DirectionComp, EmissionExtentComp, Irradiance,
    LightDirection, LightID, LightStorage, LightType, OmnidirectionalComp, OmnidirectionalLight,
    OmnidirectionalLightComp, Radiance, RadianceComp, UnidirectionalLight, UnidirectionalLightComp,
    MAX_SHADOW_MAP_CASCADES,
};
pub use material::{
    add_blinn_phong_material_component_for_entity, add_microfacet_material_component_for_entity,
    add_skybox_material_component_for_entity, register_ambient_occlusion_materials,
    register_material_components, DiffuseColorComp, DiffuseTextureComp, EmissiveColorComp,
    EmissiveTextureComp, FixedColorComp, FixedColorMaterial, FixedMaterialResources,
    FixedTextureComp, FixedTextureMaterial, MaterialComp, MaterialHandle, MaterialID,
    MaterialLibrary, MaterialPropertyTextureSet, MaterialPropertyTextureSetID,
    MaterialSpecification, MicrofacetDiffuseReflectionComp, MicrofacetSpecularReflectionComp,
    NormalMapComp, ParallaxMapComp, RGBColor, RoughnessComp, RoughnessTextureComp, SkyboxComp,
    SpecularColorComp, SpecularTextureComp, TexturedColorEmissiveMaterialFeature,
    TexturedColorParallaxMappingEmissiveMaterialFeature, UniformDiffuseEmissiveMaterialFeature,
    UniformDiffuseParallaxMappingEmissiveMaterialFeature,
    UniformDiffuseUniformSpecularEmissiveMaterialFeature,
    UniformDiffuseUniformSpecularParallaxMappingEmissiveMaterialFeature,
    UniformSpecularEmissiveMaterialFeature, UniformSpecularParallaxMappingEmissiveMaterialFeature,
    VertexColorComp, VertexColorMaterial, AMBIENT_OCCLUSION_APPLICATION_MATERIAL_ID,
    AMBIENT_OCCLUSION_APPLICATION_RENDER_PASS_HINTS, AMBIENT_OCCLUSION_COMPUTATION_MATERIAL_ID,
    AMBIENT_OCCLUSION_COMPUTATION_RENDER_PASS_HINTS, AMBIENT_OCCLUSION_DISABLED_MATERIAL_ID,
    AMBIENT_OCCLUSION_DISABLED_RENDER_PASS_HINTS, MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT,
};
pub use mesh::{
    register_mesh_components, BoxMeshComp, CircularFrustumMeshComp, ConeMeshComp, CylinderMeshComp,
    HemisphereMeshComp, MeshComp, MeshID, MeshRepository, RectangleMeshComp, SphereMeshComp,
    SCREEN_FILLING_QUAD_MESH_ID,
};
pub use model::ModelID;
pub use shader::{ShaderID, ShaderManager};
pub use systems::{
    SyncLightPositionsAndDirectionsInStorage, SyncLightRadiancesInStorage,
    SyncSceneObjectTransforms,
};
pub use tasks::{
    BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances,
    BoundUnidirectionalLightsAndBufferShadowCastingModelInstances, BufferVisibleModelInstances,
    SyncSceneCameraViewTransform, UpdateSceneGroupToWorldTransforms,
};
pub use texture_projection::{register_texture_projection_components, PlanarTextureProjectionComp};
pub use voxel::{
    register_voxel_components, VoxelBoxComp, VoxelInstanceClusterComp, VoxelManager, VoxelTreeComp,
    VoxelTreeID, VoxelTypeComp,
};

use crate::rendering::fre;
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
}

/// Global scene configuration options.
#[derive(Clone, Debug)]
pub struct SceneConfig {
    ambient_occlusion_sample_count: u32,
    ambient_occlusion_sampling_radius: fre,
    voxel_extent: fre,
}

impl Scene {
    /// Creates a new scene data container.
    pub fn new() -> Self {
        let config = SceneConfig::default();

        let mut mesh_repository = MeshRepository::new();
        mesh_repository.create_default_meshes();

        let mut instance_feature_manager = InstanceFeatureManager::new();

        let mut material_library = MaterialLibrary::new();
        material_library.register_materials(
            &mut instance_feature_manager,
            config.ambient_occlusion_sample_count,
            config.ambient_occlusion_sampling_radius,
        );

        let voxel_manager = VoxelManager::create(
            config.voxel_extent,
            &mut mesh_repository,
            &mut material_library,
            &mut instance_feature_manager,
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
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self {
            ambient_occlusion_sample_count: 4,
            ambient_occlusion_sampling_radius: 0.5,
            voxel_extent: 0.25,
        }
    }
}
