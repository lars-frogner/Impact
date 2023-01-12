//! Scene containing data to render.

mod camera;
mod components;
mod events;
mod graph;
mod instance;
mod material;
mod mesh;
mod model;
mod shader;
mod systems;
mod tasks;

pub use camera::{CameraID, CameraRepository};
pub use components::{CameraComp, MeshComp, SceneGraphNodeComp};
pub use graph::{
    model_to_world_transform_from_position_and_orientation, CameraNodeID, GroupNodeID,
    ModelInstanceNodeID, NodeStorage, NodeTransform, SceneGraph, SceneGraphNodeID,
};
pub use instance::InstanceFeatureManager;
pub use material::{
    BlinnPhongComp, BlinnPhongMaterial, DiffuseTexturedBlinnPhongComp,
    DiffuseTexturedBlinnPhongMaterial, FixedColorComp, FixedColorMaterial, MaterialComp,
    MaterialID, MaterialLibrary, MaterialSpecification, RGBAColor, RGBColor,
    TexturedBlinnPhongComp, TexturedBlinnPhongMaterial,
};
pub use mesh::{MeshID, MeshRepository};
pub use model::ModelID;
pub use shader::{ShaderID, ShaderLibrary};
pub use tasks::BufferVisibleModelInstances;

use crate::rendering::fre;
use std::sync::RwLock;

/// Container for data needed to render a scene.
#[derive(Debug)]
pub struct Scene {
    camera_repository: RwLock<CameraRepository<fre>>,
    mesh_repository: RwLock<MeshRepository<fre>>,
    shader_library: RwLock<ShaderLibrary>,
    material_library: RwLock<MaterialLibrary>,
    scene_graph: RwLock<SceneGraph<fre>>,
    instance_feature_manager: RwLock<InstanceFeatureManager>,
    active_camera: RwLock<Option<(CameraID, CameraNodeID)>>,
}

impl Scene {
    /// Creates a new scene data container.
    pub fn new(
        camera_repository: CameraRepository<fre>,
        mesh_repository: MeshRepository<fre>,
    ) -> Self {
        let scene = Self {
            camera_repository: RwLock::new(camera_repository),
            mesh_repository: RwLock::new(mesh_repository),
            shader_library: RwLock::new(ShaderLibrary::new()),
            material_library: RwLock::new(MaterialLibrary::new()),
            instance_feature_manager: RwLock::new(InstanceFeatureManager::new()),
            scene_graph: RwLock::new(SceneGraph::new()),
            active_camera: RwLock::new(None),
        };
        scene.register_materials();
        scene
    }

    /// Returns a reference to the [`CameraRepository`], guarded
    /// by a [`RwLock`].
    pub fn camera_repository(&self) -> &RwLock<CameraRepository<fre>> {
        &self.camera_repository
    }

    /// Returns a reference to the [`MeshRepository`], guarded
    /// by a [`RwLock`].
    pub fn mesh_repository(&self) -> &RwLock<MeshRepository<fre>> {
        &self.mesh_repository
    }

    /// Returns a reference to the [`ShaderLibrary`], guarded
    /// by a [`RwLock`].
    pub fn shader_library(&self) -> &RwLock<ShaderLibrary> {
        &self.shader_library
    }

    /// Returns a reference to the [`MaterialLibrary`], guarded
    /// by a [`RwLock`].
    pub fn material_library(&self) -> &RwLock<MaterialLibrary> {
        &self.material_library
    }

    /// Returns a reference to the [`InstanceFeatureManager`], guarded
    /// by a [`RwLock`].
    pub fn instance_feature_manager(&self) -> &RwLock<InstanceFeatureManager> {
        &self.instance_feature_manager
    }

    /// Returns a reference to the [`SceneGraph`], guarded
    /// by a [`RwLock`].
    pub fn scene_graph(&self) -> &RwLock<SceneGraph<fre>> {
        &self.scene_graph
    }

    /// Returns the [`CameraID`] if the currently active camera,
    /// or [`None`] if there is no active camera.
    pub fn get_active_camera_id(&self) -> Option<CameraID> {
        self.active_camera
            .read()
            .unwrap()
            .map(|(camera_id, _)| camera_id)
    }

    /// Returns the [`CameraNodeID`] if the currently active camera,
    /// or [`None`] if there is no active camera.
    pub fn get_active_camera_node_id(&self) -> Option<CameraNodeID> {
        self.active_camera
            .read()
            .unwrap()
            .map(|(_, camera_node_id)| camera_node_id)
    }

    /// Uses the camera with the given camera ID and node ID in the
    /// [`SceneGraph`] as the active camera, or disable the active
    /// camera if the value is [`None`].
    ///
    /// # Note
    /// It is the responsibility of the caller to ensure that the
    /// given combination of camera ID and node ID is valid.
    pub fn set_active_camera(&self, active_camera: Option<(CameraID, CameraNodeID)>) {
        *self.active_camera.write().unwrap() = active_camera;
    }

    fn register_materials(&self) {
        let mut shader_library = self.shader_library.write().unwrap();
        let mut material_library = self.material_library.write().unwrap();
        let mut instance_feature_manager = self.instance_feature_manager.write().unwrap();

        FixedColorMaterial::register(
            &mut shader_library,
            &mut material_library,
            &mut instance_feature_manager,
        );
        BlinnPhongMaterial::register(
            &mut shader_library,
            &mut material_library,
            &mut instance_feature_manager,
        );
        DiffuseTexturedBlinnPhongMaterial::register(
            &mut shader_library,
            &mut instance_feature_manager,
        );
        TexturedBlinnPhongMaterial::register(&mut shader_library, &mut instance_feature_manager);
    }
}
