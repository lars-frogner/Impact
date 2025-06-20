//! Running the engine.

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
