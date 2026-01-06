//! The Impact game.

pub mod api;
pub mod scripting;
pub mod user_interface;

pub use impact;

#[cfg(feature = "roc_codegen")]
pub use impact::{component::gather_roc_type_ids_for_all_components, roc_integration};

use anyhow::{Context, Result};
use dynamic_lib::DynamicLibrary;
use impact::{
    application::Application,
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
use impact_dev_ui::UserInterfaceConfig;
use parking_lot::RwLock;
use scripting::ScriptLib;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use user_interface::UserInterface;

static ENGINE: RwLock<Option<Arc<Engine>>> = RwLock::new(None);

#[derive(Debug)]
pub struct Game {
    game_options: RwLock<GameOptions>,
    user_interface: RwLock<UserInterface>,
    #[cfg(feature = "hot_reloading")]
    script_reloader: RwLock<Option<scripting::hot_reloading::ScriptReloader>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GameConfig {
    pub game_options: GameOptions,
    pub run_mode: RunMode,
    pub window: WindowConfig,
    pub runtime: RuntimeConfig,
    pub engine_config_path: PathBuf,
    pub ui_config_path: PathBuf,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct GameOptions {
    reset_scene_on_reload: bool,
    #[serde(skip)]
    scene_reset_requested: bool,
    #[serde(skip)]
    show_game_options: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunMode {
    #[default]
    Windowed,
    Headless,
}

impl Game {
    pub fn new(game_options: GameOptions, user_interface: UserInterface) -> Self {
        Self {
            game_options: RwLock::new(game_options),
            user_interface: RwLock::new(user_interface),
            #[cfg(feature = "hot_reloading")]
            script_reloader: Default::default(),
        }
    }

    #[cfg(feature = "hot_reloading")]
    fn activate_script_reloader(&self) -> Result<()> {
        log::debug!("Activating script reloader");

        let script_reloader = scripting::hot_reloading::create_script_reloader()?;
        *self.script_reloader.write() = Some(script_reloader);

        Ok(())
    }

    #[cfg(not(feature = "hot_reloading"))]
    #[allow(clippy::unused_self)]
    fn activate_script_reloader(&self) -> Result<()> {
        Ok(())
    }

    #[cfg(feature = "hot_reloading")]
    fn respond_to_script_reload(&self, engine: &Engine) -> Result<()> {
        let script_reloader = self.script_reloader.read();

        let was_reloaded = script_reloader
            .as_ref()
            .unwrap()
            .reloaded_since_last_check();

        let options = self.game_options.read();

        if was_reloaded && options.reset_scene_on_reload && !options.scene_reset_requested {
            Self::reset_scene(engine)?;
        }

        Ok(())
    }

    #[cfg(not(feature = "hot_reloading"))]
    #[allow(clippy::unused_self)]
    fn respond_to_script_reload(&self, _engine: &Engine) -> Result<()> {
        Ok(())
    }

    fn perform_requested_option_actions(&self, engine: &Engine) -> Result<()> {
        let mut options = self.game_options.upgradable_read();

        if options.scene_reset_requested {
            options.with_upgraded(|opts| {
                opts.scene_reset_requested = false;
            });
            Self::reset_scene(engine)?;
        }

        Ok(())
    }

    fn reset_scene(engine: &Engine) -> Result<()> {
        log::debug!("Resetting scene");
        engine.reset_world();
        scripting::setup_scene()
    }
}

impl Application for Game {
    fn on_engine_initialized(&self, engine: Arc<Engine>) -> Result<()> {
        log::debug!("Loading script library");
        ScriptLib::load().context("Failed to load script library")?;

        self.activate_script_reloader()?;

        *ENGINE.write() = Some(engine.clone());
        log::debug!("Engine initialized");

        log::debug!("Setting up UI");
        self.user_interface.read().setup(&engine);

        log::debug!("Setting up scene");
        scripting::setup_scene()?;

        Ok(())
    }

    fn on_new_frame(&self, engine: &Engine, _frame_number: u64) -> Result<()> {
        self.respond_to_script_reload(engine)?;
        self.perform_requested_option_actions(engine)?;
        Ok(())
    }

    fn handle_keyboard_event(&self, event: KeyboardEvent) -> Result<()> {
        log::trace!("Handling keyboard event {event:?}");
        scripting::handle_keyboard_event(event)
    }

    fn handle_mouse_button_event(&self, event: MouseButtonEvent) -> Result<()> {
        log::trace!("Handling mouse button event {event:?}");
        scripting::handle_mouse_button_event(event)
    }

    fn handle_mouse_drag_event(&self, event: MouseDragEvent) -> Result<()> {
        log::trace!("Handling mouse drag event {event:?}");
        scripting::handle_mouse_drag_event(event)
    }

    fn handle_mouse_scroll_event(&self, event: MouseScrollEvent) -> Result<()> {
        log::trace!("Handling mouse scroll event {event:?}");
        scripting::handle_mouse_scroll_event(event)
    }

    fn run_egui_ui(
        &self,
        ctx: &egui::Context,
        input: egui::RawInput,
        engine: &Engine,
    ) -> egui::FullOutput {
        self.user_interface.write().run(
            ctx,
            input,
            engine,
            &api::UI_COMMANDS,
            &mut self.game_options.write(),
        )
    }
}

impl GameConfig {
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
        GameOptions,
        RunMode,
        WindowConfig,
        RuntimeConfig,
        EngineConfig,
        UserInterfaceConfig,
    )> {
        let Self {
            game_options,
            run_mode,
            window,
            runtime,
            engine_config_path,
            ui_config_path,
        } = self;

        let engine = EngineConfig::from_ron_file(engine_config_path)?;
        let ui = UserInterfaceConfig::from_ron_file(ui_config_path)?;

        Ok((game_options, run_mode, window, runtime, engine, ui))
    }

    /// Resolves all paths in the configuration by prepending the given root
    /// path to all paths.
    fn resolve_paths(&mut self, root_path: &Path) {
        self.engine_config_path = root_path.join(&self.engine_config_path);
        self.ui_config_path = root_path.join(&self.ui_config_path);
    }
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            game_options: GameOptions::default(),
            run_mode: RunMode::default(),
            window: WindowConfig::default(),
            runtime: RuntimeConfig::default(),
            engine_config_path: PathBuf::from("engine_config.roc"),
            ui_config_path: PathBuf::from("ui_config.roc"),
        }
    }
}
