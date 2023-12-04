//! Scene containing data to render.

mod camera;
mod components;
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

pub use camera::{OrthographicCameraComp, PerspectiveCameraComp, SceneCamera};
pub use components::{
    ParentComp, ScalingComp, SceneGraphCameraNodeComp, SceneGraphGroup, SceneGraphGroupNodeComp,
    SceneGraphModelInstanceNodeComp, SceneGraphNodeComp, SceneGraphParentNodeComp, Uncullable,
};
pub use events::RenderResourcesDesynchronized;
pub use graph::{
    create_child_to_parent_transform, CameraNodeID, GroupNodeID, ModelInstanceNodeID, NodeStorage,
    NodeTransform, SceneGraph, SceneGraphNodeID,
};
pub use instance::InstanceFeatureManager;
pub use light::{
    AmbientLight, AmbientLightComp, AngularExtentComp, DirectionComp, EmissionExtentComp,
    Irradiance, LightDirection, LightID, LightStorage, LightType, Omnidirectional,
    OmnidirectionalLight, OmnidirectionalLightComp, Radiance, RadianceComp, UnidirectionalLight,
    UnidirectionalLightComp, UniformIrradianceComp, MAX_SHADOW_MAP_CASCADES,
};
pub use material::{
    add_blinn_phong_material_component_for_entity, add_microfacet_material_component_for_entity,
    add_skybox_material_component_for_entity, register_ambient_occlusion_materials,
    DiffuseColorComp, DiffuseTextureComp, EmissiveColorComp, EmissiveTextureComp, FixedColorComp,
    FixedColorMaterial, FixedMaterialResources, FixedTextureComp, FixedTextureMaterial,
    MaterialComp, MaterialHandle, MaterialID, MaterialLibrary, MaterialPropertyTextureSet,
    MaterialPropertyTextureSetID, MaterialSpecification, MicrofacetDiffuseReflection,
    MicrofacetSpecularReflection, NormalMapComp, ParallaxMapComp, RGBColor, RoughnessComp,
    RoughnessTextureComp, SkyboxComp, SpecularColorComp, SpecularTextureComp,
    TexturedColorEmissiveMaterialFeature, TexturedColorParallaxMappingEmissiveMaterialFeature,
    UniformDiffuseEmissiveMaterialFeature, UniformDiffuseParallaxMappingEmissiveMaterialFeature,
    UniformDiffuseUniformSpecularEmissiveMaterialFeature,
    UniformDiffuseUniformSpecularParallaxMappingEmissiveMaterialFeature,
    UniformSpecularEmissiveMaterialFeature, UniformSpecularParallaxMappingEmissiveMaterialFeature,
    VertexColorComp, VertexColorMaterial, AMBIENT_OCCLUSION_APPLICATION_MATERIAL_ID,
    AMBIENT_OCCLUSION_APPLICATION_RENDER_PASS_HINTS, AMBIENT_OCCLUSION_COMPUTATION_MATERIAL_ID,
    AMBIENT_OCCLUSION_COMPUTATION_RENDER_PASS_HINTS, AMBIENT_OCCLUSION_DISABLED_MATERIAL_ID,
    AMBIENT_OCCLUSION_DISABLED_RENDER_PASS_HINTS, MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT,
};
pub use mesh::{
    BoxMeshComp, CircularFrustumMeshComp, ConeMeshComp, CylinderMeshComp, HemisphereMeshComp,
    MeshComp, MeshID, MeshRepository, RectangleMeshComp, SphereMeshComp,
    SCREEN_FILLING_QUAD_MESH_ID,
};
pub use model::ModelID;
pub use shader::{ShaderID, ShaderManager};
pub use systems::{SyncLightPositionsAndDirectionsInStorage, SyncSceneObjectTransforms};
pub use tasks::{
    BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances,
    BoundUnidirectionalLightsAndBufferShadowCastingModelInstances, BufferVisibleModelInstances,
    SyncSceneCameraViewTransform, UpdateSceneGroupToWorldTransforms,
};
pub use texture_projection::PlanarTextureProjectionComp;

use crate::rendering::fre;
use material::{
    TexturedColorMaterialFeature, TexturedColorParallaxMappingMaterialFeature,
    UniformDiffuseMaterialFeature, UniformDiffuseParallaxMappingMaterialFeature,
    UniformDiffuseUniformSpecularMaterialFeature,
    UniformDiffuseUniformSpecularParallaxMappingMaterialFeature, UniformSpecularMaterialFeature,
    UniformSpecularParallaxMappingMaterialFeature,
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
    scene_graph: RwLock<SceneGraph<fre>>,
    scene_camera: RwLock<Option<SceneCamera<fre>>>,
}

/// Global scene configuration options.
#[derive(Clone, Debug)]
pub struct SceneConfig {
    ambient_occlusion_sample_count: u32,
    ambient_occlusion_sampling_radius: fre,
}

impl Scene {
    /// Creates a new scene data container.
    pub fn new() -> Self {
        let config = SceneConfig::default();

        let scene = Self {
            config,
            mesh_repository: RwLock::new(MeshRepository::new()),
            material_library: RwLock::new(MaterialLibrary::new()),
            light_storage: RwLock::new(LightStorage::new()),
            instance_feature_manager: RwLock::new(InstanceFeatureManager::new()),
            shader_manager: RwLock::new(ShaderManager::new()),
            scene_graph: RwLock::new(SceneGraph::new()),
            scene_camera: RwLock::new(None),
        };

        scene.register_materials();

        scene
    }

    /// Returns a reference to the global scene configuration.
    pub fn config(&self) -> &SceneConfig {
        &self.config
    }

    /// Returns a reference to the [`MeshRepository`], guarded
    /// by a [`RwLock`].
    pub fn mesh_repository(&self) -> &RwLock<MeshRepository<fre>> {
        &self.mesh_repository
    }

    /// Returns a reference to the [`MaterialLibrary`], guarded
    /// by a [`RwLock`].
    pub fn material_library(&self) -> &RwLock<MaterialLibrary> {
        &self.material_library
    }

    /// Returns a reference to the [`LightStorage`], guarded
    /// by a [`RwLock`].
    pub fn light_storage(&self) -> &RwLock<LightStorage> {
        &self.light_storage
    }

    /// Returns a reference to the [`InstanceFeatureManager`], guarded
    /// by a [`RwLock`].
    pub fn instance_feature_manager(&self) -> &RwLock<InstanceFeatureManager> {
        &self.instance_feature_manager
    }

    /// Returns a reference to the [`ShaderManager`], guarded
    /// by a [`RwLock`].
    pub fn shader_manager(&self) -> &RwLock<ShaderManager> {
        &self.shader_manager
    }

    /// Returns a reference to the [`SceneGraph`], guarded
    /// by a [`RwLock`].
    pub fn scene_graph(&self) -> &RwLock<SceneGraph<fre>> {
        &self.scene_graph
    }

    /// Returns a reference to the [`SceneCamera`], or [`None`] if no
    /// scene camera has been set, guarded by a [`RwLock`].
    pub fn scene_camera(&self) -> &RwLock<Option<SceneCamera<fre>>> {
        &self.scene_camera
    }

    fn register_materials(&self) {
        let mut material_library = self.material_library.write().unwrap();
        let mut instance_feature_manager = self.instance_feature_manager.write().unwrap();

        instance_feature_manager.register_feature_type::<TexturedColorMaterialFeature>();
        instance_feature_manager.register_feature_type::<UniformDiffuseMaterialFeature>();
        instance_feature_manager.register_feature_type::<UniformSpecularMaterialFeature>();
        instance_feature_manager
            .register_feature_type::<UniformDiffuseUniformSpecularMaterialFeature>();
        instance_feature_manager
            .register_feature_type::<TexturedColorParallaxMappingMaterialFeature>();
        instance_feature_manager
            .register_feature_type::<UniformDiffuseParallaxMappingMaterialFeature>();
        instance_feature_manager
            .register_feature_type::<UniformSpecularParallaxMappingMaterialFeature>();
        instance_feature_manager
            .register_feature_type::<UniformDiffuseUniformSpecularParallaxMappingMaterialFeature>();
        instance_feature_manager.register_feature_type::<TexturedColorEmissiveMaterialFeature>();
        instance_feature_manager.register_feature_type::<UniformDiffuseEmissiveMaterialFeature>();
        instance_feature_manager.register_feature_type::<UniformSpecularEmissiveMaterialFeature>();
        instance_feature_manager
            .register_feature_type::<UniformDiffuseUniformSpecularEmissiveMaterialFeature>();
        instance_feature_manager
            .register_feature_type::<TexturedColorParallaxMappingEmissiveMaterialFeature>();
        instance_feature_manager
            .register_feature_type::<UniformDiffuseParallaxMappingEmissiveMaterialFeature>();
        instance_feature_manager
            .register_feature_type::<UniformSpecularParallaxMappingEmissiveMaterialFeature>();
        instance_feature_manager
            .register_feature_type::<UniformDiffuseUniformSpecularParallaxMappingEmissiveMaterialFeature>();

        VertexColorMaterial::register(&mut material_library);
        FixedColorMaterial::register(&mut material_library, &mut instance_feature_manager);
        FixedTextureMaterial::register(&mut material_library);

        register_ambient_occlusion_materials(
            &mut material_library,
            self.config.ambient_occlusion_sample_count,
            self.config.ambient_occlusion_sampling_radius,
        );
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
        }
    }
}
