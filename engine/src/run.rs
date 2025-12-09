//! Running the engine.

pub mod headless {
    use crate::{
        application::Application,
        engine::{Engine, EngineConfig},
        gpu,
        runtime::{
            Runtime, RuntimeConfig,
            headless::{HeadlessConfig, HeadlessRuntime, run_headless},
        },
    };
    use anyhow::Result;
    use std::sync::Arc;

    pub fn run(
        app: Arc<dyn Application>,
        headless_config: HeadlessConfig,
        runtime_config: RuntimeConfig,
        engine_config: EngineConfig,
    ) -> Result<()> {
        let runtime = create_runtime(app, headless_config, runtime_config, engine_config)?;
        run_headless(runtime)
    }

    fn create_runtime(
        app: Arc<dyn Application>,
        headless_config: HeadlessConfig,
        runtime_config: RuntimeConfig,
        engine_config: EngineConfig,
    ) -> Result<HeadlessRuntime> {
        let (width, height) = headless_config.surface_size;
        let graphics = gpu::initialize_for_headless_rendering(width, height)?;

        let engine = Engine::new(engine_config, app, graphics)?;

        let runtime = Runtime::new_without_ui(engine, runtime_config)?;

        runtime
            .engine()
            .app()
            .on_engine_initialized(runtime.arc_engine())?;

        Ok(runtime)
    }
}

#[cfg(feature = "egui")]
pub mod window {
    use crate::{
        application::Application,
        engine::{Engine, EngineConfig},
        gpu,
        runtime::{Runtime, RuntimeConfig, window::WindowRuntimeHandler},
        ui::egui::{EguiUserInterface, EguiUserInterfaceConfig},
        window::{Window, WindowConfig},
    };
    use anyhow::Result;
    use std::sync::Arc;

    pub fn run(
        app: Arc<dyn Application>,
        window_config: WindowConfig,
        runtime_config: RuntimeConfig,
        engine_config: EngineConfig,
    ) -> Result<()> {
        let mut runtime_handler = WindowRuntimeHandler::new(
            |window| create_runtime(app, window, runtime_config, engine_config),
            window_config,
        );
        runtime_handler.run()
    }

    fn create_runtime(
        app: Arc<dyn Application>,
        window: Window,
        runtime_config: RuntimeConfig,
        engine_config: EngineConfig,
    ) -> Result<Runtime<EguiUserInterface>> {
        let graphics = gpu::initialize_for_window_rendering(&window)?;

        let engine = Engine::new(engine_config, app.clone(), graphics)?;

        let user_interface =
            EguiUserInterface::new(EguiUserInterfaceConfig::default(), app, &engine, window);

        let runtime = Runtime::new(engine, user_interface, runtime_config)?;

        runtime
            .engine()
            .app()
            .on_engine_initialized(runtime.arc_engine())?;

        Ok(runtime)
    }
}
