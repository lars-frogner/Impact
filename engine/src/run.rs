//! Running the engine.

use crate::{
    application::Application,
    engine::{Engine, EngineConfig},
    runtime::{Runtime, RuntimeConfig, window::WindowRuntimeHandler},
    ui::UserInterface,
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
) -> Result<Runtime> {
    let engine = Engine::new(engine_config, app.clone(), window.clone())?;
    let user_interface = UserInterface::new(app, window);
    let runtime = Runtime::new(engine, user_interface, runtime_config)?;
    on_engine_created(runtime.arc_engine());
    runtime.engine().app().setup_scene()?;
    Ok(runtime)
}
