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
pub use components::{ScalingComp, SceneGraphCameraNodeComp, SceneGraphNodeComp};
pub use events::RenderResourcesDesynchronized;
pub use graph::{
    create_model_to_world_transform, CameraNodeID, GroupNodeID, ModelInstanceNodeID, NodeStorage,
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
    DiffuseColorComp, DiffuseTextureComp, FixedColorComp, FixedColorMaterial,
    FixedMaterialResources, FixedTextureComp, FixedTextureMaterial, MaterialComp, MaterialHandle,
    MaterialID, MaterialLibrary, MaterialPropertyTextureSet, MaterialPropertyTextureSetID,
    MaterialSpecification, MicrofacetDiffuseReflection, MicrofacetSpecularReflection,
    NormalMapComp, ParallaxMapComp, ParallaxMappingPrepassMaterialFeature, RGBColor, RoughnessComp,
    RoughnessTextureComp, SpecularColorComp, SpecularTextureComp,
    TexturedColorBlinnPhongMaterialFeature, TexturedColorMicrofacetMaterialFeature,
    UniformColorBlinnPhongMaterialFeature, UniformColorMicrofacetMaterialFeature,
    UniformDiffuseBlinnPhongMaterialFeature, UniformDiffuseMicrofacetMaterialFeature,
    UniformDiffuseParallaxMappingPrepassMaterialFeature, UniformDiffusePrepassMaterialFeature,
    UniformSpecularBlinnPhongMaterialFeature, UniformSpecularMicrofacetMaterialFeature,
    VertexColorComp, VertexColorMaterial,
};
pub use mesh::{
    BoxMeshComp, CylinderMeshComp, MeshComp, MeshID, MeshRepository, PlaneMeshComp, SphereMeshComp,
};
pub use model::ModelID;
pub use shader::{ShaderID, ShaderManager};
pub use systems::SyncLightPositionsAndDirectionsInStorage;
pub use tasks::{
    BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances,
    BoundUnidirectionalLightsAndBufferShadowCastingModelInstances, BufferVisibleModelInstances,
    SyncSceneCameraViewTransform,
};
pub use texture_projection::PlanarTextureProjectionComp;

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
    scene_graph: RwLock<SceneGraph<fre>>,
    scene_camera: RwLock<Option<SceneCamera<fre>>>,
}

/// Global scene configuration options.
#[derive(Clone, Debug, Default)]
pub struct SceneConfig {}

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

        instance_feature_manager.register_feature_type::<UniformColorBlinnPhongMaterialFeature>();
        instance_feature_manager.register_feature_type::<UniformDiffuseBlinnPhongMaterialFeature>();
        instance_feature_manager
            .register_feature_type::<UniformSpecularBlinnPhongMaterialFeature>();
        instance_feature_manager.register_feature_type::<TexturedColorBlinnPhongMaterialFeature>();

        instance_feature_manager.register_feature_type::<UniformColorMicrofacetMaterialFeature>();
        instance_feature_manager.register_feature_type::<UniformDiffuseMicrofacetMaterialFeature>();
        instance_feature_manager
            .register_feature_type::<UniformSpecularMicrofacetMaterialFeature>();
        instance_feature_manager.register_feature_type::<TexturedColorMicrofacetMaterialFeature>();

        instance_feature_manager.register_feature_type::<UniformDiffusePrepassMaterialFeature>();
        instance_feature_manager.register_feature_type::<ParallaxMappingPrepassMaterialFeature>();
        instance_feature_manager
            .register_feature_type::<UniformDiffuseParallaxMappingPrepassMaterialFeature>();

        VertexColorMaterial::register(&mut material_library);
        FixedColorMaterial::register(&mut material_library, &mut instance_feature_manager);
        FixedTextureMaterial::register(&mut material_library);
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}
