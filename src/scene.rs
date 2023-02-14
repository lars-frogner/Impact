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

pub use camera::{PerspectiveCameraComp, SceneCamera};
pub use components::{MeshComp, ScalingComp, SceneGraphCameraNodeComp, SceneGraphNodeComp};
pub use events::RenderResourcesDesynchronized;
pub use graph::{
    create_model_to_world_transform, CameraNodeID, GroupNodeID, ModelInstanceNodeID, NodeStorage,
    NodeTransform, SceneGraph, SceneGraphNodeID,
};
pub use instance::InstanceFeatureManager;
pub use light::{
    LightID, LightStorage, Omnidirectional, PointLight, PointLightComp, Radiance, RadianceComp,
};
pub use material::{
    BlinnPhongComp, BlinnPhongMaterial, DiffuseTexturedBlinnPhongComp,
    DiffuseTexturedBlinnPhongMaterial, FixedColorComp, FixedColorMaterial, FixedTextureComp,
    FixedTextureMaterial, MaterialComp, MaterialID, MaterialLibrary, MaterialSpecification,
    RGBAColor, RGBColor, TexturedBlinnPhongComp, TexturedBlinnPhongMaterial, VertexColorComp,
    VertexColorMaterial,
};
pub use mesh::{MeshID, MeshRepository};
pub use model::ModelID;
pub use shader::{ShaderID, ShaderManager};
pub use systems::SyncLightPositionsInStorage;
pub use tasks::{BufferVisibleModelInstances, SyncSceneCameraViewTransform};

use crate::rendering::fre;
use std::sync::RwLock;

/// Container for data needed to render a scene.
#[derive(Debug)]
pub struct Scene {
    mesh_repository: RwLock<MeshRepository<fre>>,
    material_library: RwLock<MaterialLibrary>,
    light_storage: RwLock<LightStorage>,
    instance_feature_manager: RwLock<InstanceFeatureManager>,
    shader_manager: RwLock<ShaderManager>,
    scene_graph: RwLock<SceneGraph<fre>>,
    scene_camera: RwLock<Option<SceneCamera<fre>>>,
}

impl Scene {
    /// Creates a new scene data container.
    pub fn new(mesh_repository: MeshRepository<fre>) -> Self {
        let scene = Self {
            mesh_repository: RwLock::new(mesh_repository),
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

        VertexColorMaterial::register(&mut material_library);
        FixedColorMaterial::register(&mut material_library, &mut instance_feature_manager);
        BlinnPhongMaterial::register(&mut material_library, &mut instance_feature_manager);
        DiffuseTexturedBlinnPhongMaterial::register(&mut instance_feature_manager);
        TexturedBlinnPhongMaterial::register(&mut instance_feature_manager);
    }
}
