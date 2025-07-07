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
    use std::{num::NonZeroU32, sync::Arc};

    pub fn run(
        app: Arc<dyn Application>,
        headless_config: HeadlessConfig,
        runtime_config: RuntimeConfig,
        engine_config: EngineConfig,
        on_engine_created: impl FnOnce(Arc<Engine>) + 'static,
    ) -> Result<()> {
        let HeadlessConfig {
            surface_size,
            actions,
            termination_criterion,
        } = headless_config;

        let runtime = create_runtime(
            app,
            surface_size,
            runtime_config,
            engine_config,
            on_engine_created,
        )?;

        run_headless(runtime, actions, termination_criterion)
    }

    fn create_runtime(
        app: Arc<dyn Application>,
        surface_size: (NonZeroU32, NonZeroU32),
        runtime_config: RuntimeConfig,
        engine_config: EngineConfig,
        on_engine_created: impl FnOnce(Arc<Engine>),
    ) -> Result<HeadlessRuntime> {
        let (width, height) = surface_size;
        let graphics = gpu::initialize_for_headless_rendering(width, height)?;

        let engine = Engine::new(engine_config, app.clone(), graphics)?;

        let runtime = Runtime::new_without_ui(engine, runtime_config)?;

        on_engine_created(runtime.arc_engine());

        runtime.engine().app().setup_scene()?;

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
        on_engine_created: impl FnOnce(Arc<Engine>) + 'static,
    ) -> Result<()> {
        let mut runtime_handler = WindowRuntimeHandler::new(
            |window| {
                create_runtime(
                    app,
                    window,
                    runtime_config,
                    engine_config,
                    on_engine_created,
                )
            },
            window_config,
        );
        runtime_handler.run()
    }

    fn create_runtime(
        app: Arc<dyn Application>,
        window: Window,
        runtime_config: RuntimeConfig,
        engine_config: EngineConfig,
        on_engine_created: impl FnOnce(Arc<Engine>),
    ) -> Result<Runtime<EguiUserInterface>> {
        let graphics = gpu::initialize_for_window_rendering(&window)?;

        let engine = Engine::new(engine_config, app.clone(), graphics)?;

        let user_interface =
            EguiUserInterface::new(EguiUserInterfaceConfig::default(), app, &engine, window);

        let runtime = Runtime::new(engine, user_interface, runtime_config)?;

        on_engine_created(runtime.arc_engine());

        runtime.engine().app().setup_scene()?;

        Ok(runtime)
    }
}
