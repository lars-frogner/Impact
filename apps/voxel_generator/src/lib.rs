//! An editor for generated voxel objects.

pub mod api;
pub mod editor;
pub mod scripting;

pub use impact;

#[cfg(feature = "roc_codegen")]
pub use impact::{component::gather_roc_type_ids_for_all_components, roc_integration};

use anyhow::Result;
use editor::{Editor, EditorConfig};
use impact::{
    application::Application,
    bumpalo::Bump,
    egui,
    engine::{Engine, EngineConfig},
    impact_io,
    input::{
        key::KeyboardEvent,
        mouse::{MouseButtonEvent, MouseDragEvent, MouseScrollEvent},
    },
    runtime::RuntimeConfig,
    window::WindowConfig,
};
use impact_dev_ui::{UICommandQueue, UserInterface as DevUserInterface, UserInterfaceConfig};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

static ENGINE: RwLock<Option<Arc<Engine>>> = RwLock::new(None);

#[derive(Debug)]
pub struct VoxelGenerator {
    user_interface: RwLock<UserInterface>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct VoxelGeneratorConfig {
    pub editor: EditorConfig,
    pub window: WindowConfig,
    pub runtime: RuntimeConfig,
    pub engine_config_path: PathBuf,
    pub ui_config_path: PathBuf,
}

#[derive(Debug)]
pub struct UserInterface {
    editor: Editor,
    dev_ui: DevUserInterface,
}

impl VoxelGenerator {
    pub fn new(user_interface: UserInterface) -> Self {
        Self {
            user_interface: RwLock::new(user_interface),
        }
    }
}

impl Application for VoxelGenerator {
    fn on_engine_initialized(&self, arena: &Bump, engine: Arc<Engine>) -> Result<()> {
        *ENGINE.write() = Some(engine.clone());
        impact_log::debug!("Engine initialized");

        impact_log::debug!("Setting up UI");
        self.user_interface.read().setup(&engine);

        impact_log::debug!("Setting up scene");
        scripting::setup_scene()
    }

    fn handle_keyboard_event(&self, _arena: &Bump, event: KeyboardEvent) -> Result<()> {
        impact_log::trace!("Handling keyboard event {event:?}");
        scripting::handle_keyboard_event(event)
    }

    fn handle_mouse_button_event(&self, _arena: &Bump, event: MouseButtonEvent) -> Result<()> {
        impact_log::trace!("Handling mouse button event {event:?}");
        scripting::handle_mouse_button_event(event)
    }

    fn handle_mouse_drag_event(&self, _arena: &Bump, event: MouseDragEvent) -> Result<()> {
        impact_log::trace!("Handling mouse drag event {event:?}");
        scripting::handle_mouse_drag_event(event)
    }

    fn handle_mouse_scroll_event(&self, _arena: &Bump, event: MouseScrollEvent) -> Result<()> {
        impact_log::trace!("Handling mouse scroll event {event:?}");
        scripting::handle_mouse_scroll_event(event)
    }

    fn run_egui_ui(
        &self,
        _arena: &Bump,
        ctx: &egui::Context,
        input: egui::RawInput,
        engine: &Engine,
    ) -> egui::FullOutput {
        self.user_interface
            .write()
            .run(ctx, input, engine, &api::UI_COMMANDS)
    }
}

impl VoxelGeneratorConfig {
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

impl Default for VoxelGeneratorConfig {
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

impl UserInterface {
    pub fn new(editor: Editor, dev_ui: DevUserInterface) -> Self {
        Self { editor, dev_ui }
    }

    pub fn setup(&self, engine: &Engine) {
        self.dev_ui.setup(engine);
    }

    pub fn run(
        &mut self,
        ctx: &egui::Context,
        input: egui::RawInput,
        engine: &Engine,
        command_queue: &UICommandQueue,
    ) -> egui::FullOutput {
        self.dev_ui
            .run_with_custom_panels(ctx, input, engine, command_queue, &mut self.editor)
    }
}
