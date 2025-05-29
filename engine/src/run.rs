//! Running the engine.

use crate::{
    application::Application,
    engine::Engine,
    runtime::{Runtime, RuntimeHandler},
    ui::UserInterface,
    window::Window,
};
use anyhow::Result;
use std::sync::Arc;

pub fn run(
    app: Arc<dyn Application>,
    on_engine_created: impl FnOnce(Arc<Engine>) + 'static,
) -> Result<()> {
    let window_config = app.window_config();
    let mut runtime_invoker = RuntimeHandler::new(
        |window| create_runtime(app, window, on_engine_created),
        window_config,
    );
    runtime_invoker.run()
}

fn create_runtime(
    app: Arc<dyn Application>,
    window: Window,
    on_engine_created: impl FnOnce(Arc<Engine>),
) -> Result<Runtime> {
    let runtime_config = app.runtime_config();
    let engine = Engine::new(app.clone(), window.clone())?;
    let user_interface = UserInterface::new(app, window);
    let runtime = Runtime::new(engine, user_interface, runtime_config)?;
    on_engine_created(runtime.arc_engine());
    runtime.engine().app().setup_scene()?;
    Ok(runtime)
}
