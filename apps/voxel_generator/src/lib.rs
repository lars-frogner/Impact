//! An editor for generated voxel objects.

pub mod editor;
pub mod interface;
pub mod user_interface;

pub use impact;

#[cfg(feature = "roc_codegen")]
pub use impact::{component::gather_roc_type_ids_for_all_components, roc_integration};

use anyhow::Result;
use editor::{Editor, EditorConfig};
use impact::{
    engine::{Engine, EngineConfig},
    impact_alloc::{Allocator, Global},
    impact_geometry::{ModelTransform, ReferenceFrame},
    impact_id::EntityID,
    impact_io,
    impact_thread::pool::{DynamicThreadPool, ThreadPool},
    runtime::RuntimeConfig,
    window::WindowConfig,
};
use impact_dev_ui::UserInterfaceConfig;
use impact_voxel::{
    HasVoxelObject,
    chunks::ChunkedVoxelObject,
    generation::{ChunkedVoxelGenerator, SDFVoxelGenerator},
    mesh::MeshedChunkedVoxelObject,
};
use serde::{Deserialize, Serialize};
use std::{
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::Arc,
};
use user_interface::UserInterface;

const OBJECT_ENTITY_ID: EntityID = EntityID::hashed_from_str("object");

#[derive(Debug)]
pub struct App {
    user_interface: UserInterface,
    thread_pool: DynamicThreadPool,
    engine: Option<Arc<Engine>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub editor: EditorConfig,
    pub window: WindowConfig,
    pub runtime: RuntimeConfig,
    pub engine_config_path: PathBuf,
    pub ui_config_path: PathBuf,
}

impl App {
    pub(crate) fn new(user_interface: UserInterface) -> Self {
        let n_workers = num_threads();
        let queue_capacity = NonZeroUsize::new(n_workers.get() * 64).unwrap();
        Self {
            user_interface,
            thread_pool: ThreadPool::new_dynamic(n_workers, queue_capacity),
            engine: None,
        }
    }

    pub(crate) fn engine(&self) -> &Engine {
        self.engine
            .as_ref()
            .expect("Tried to use engine before initialization")
    }

    fn initialize_voxel_object(&mut self) -> Result<()> {
        let (voxel_object, model_transform) = generate_next_voxel_object_or_default(
            &self.thread_pool,
            self.user_interface.editor_mut(),
        );

        self.engine()
            .add_voxel_object(OBJECT_ENTITY_ID, voxel_object)?;

        self.engine().create_entity_with_id(
            OBJECT_ENTITY_ID,
            (
                &HasVoxelObject,
                &model_transform,
                &ReferenceFrame::unoriented([0.0; 3].into()),
            ),
        )
    }

    fn update_voxel_object(&mut self) -> Result<()> {
        if let Some((voxel_object, new_model_transform)) =
            generate_next_voxel_object(&self.thread_pool, self.user_interface.editor_mut())
        {
            let engine = self.engine();

            engine.with_component_mut(OBJECT_ENTITY_ID, |model_transform| {
                *model_transform = new_model_transform;
                Ok(())
            })?;
            engine.replace_voxel_object(OBJECT_ENTITY_ID, voxel_object);
        }
        Ok(())
    }
}

impl AppConfig {
    /// Parses the configuration from the RON file at the given path and
    /// resolves any specified paths.
    pub fn from_ron_file(file_path: impl AsRef<Path>) -> Result<Self> {
        let file_path = file_path.as_ref();
        let mut config: Self = impact_io::parse_ron_file(file_path)?;
        if let Some(root_path) = file_path.parent() {
            config.resolve_paths(root_path);
        }
        Ok(config)
    }

    pub fn load(
        self,
    ) -> Result<(
        EditorConfig,
        WindowConfig,
        RuntimeConfig,
        EngineConfig,
        UserInterfaceConfig,
    )> {
        let Self {
            editor,
            window,
            runtime,
            engine_config_path,
            ui_config_path,
        } = self;

        let engine = EngineConfig::from_ron_file(engine_config_path)?;
        let dev_ui = UserInterfaceConfig::from_ron_file(ui_config_path)?;

        Ok((editor, window, runtime, engine, dev_ui))
    }

    /// Resolves all paths in the configuration by prepending the given root
    /// path to all paths.
    fn resolve_paths(&mut self, root_path: &Path) {
        self.engine_config_path = root_path.join(&self.engine_config_path);
        self.ui_config_path = root_path.join(&self.ui_config_path);
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            editor: EditorConfig::default(),
            window: WindowConfig::default(),
            runtime: RuntimeConfig::default(),
            engine_config_path: PathBuf::from("engine_config.roc"),
            ui_config_path: PathBuf::from("ui_config.roc"),
        }
    }
}

fn generate_next_voxel_object(
    thread_pool: &DynamicThreadPool,
    editor: &mut Editor,
) -> Option<(MeshedChunkedVoxelObject, ModelTransform)> {
    let generator = editor.build_next_voxel_sdf_generator(Global)?;
    Some((
        MeshedChunkedVoxelObject::create(ChunkedVoxelObject::generate_in_parallel(
            thread_pool,
            &generator,
        )),
        compute_model_transform(&generator),
    ))
}

fn generate_next_voxel_object_or_default(
    thread_pool: &DynamicThreadPool,
    editor: &mut Editor,
) -> (MeshedChunkedVoxelObject, ModelTransform) {
    let generator = editor.build_next_voxel_sdf_generator_or_default(Global);
    (
        MeshedChunkedVoxelObject::create(ChunkedVoxelObject::generate_in_parallel(
            thread_pool,
            &generator,
        )),
        compute_model_transform(&generator),
    )
}

fn compute_model_transform<A: Allocator>(generator: &SDFVoxelGenerator<A>) -> ModelTransform {
    ModelTransform::with_offset(
        generator.voxel_extent() * generator.grid_center().as_vector().compact(),
    )
}

fn num_threads() -> NonZeroUsize {
    std::thread::available_parallelism().unwrap_or_else(|_| NonZeroUsize::new(4).unwrap())
}
