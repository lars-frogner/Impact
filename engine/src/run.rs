//! Running the engine.

use crate::{
    application::Application,
    engine::Engine,
    game_loop::GameLoop,
    window::{GameHandler, Window},
};
use anyhow::Result;
use std::sync::Arc;

pub fn run(
    app: Arc<dyn Application>,
    on_engine_created: impl FnOnce(Arc<Engine>) + 'static,
) -> Result<()> {
    let mut handler = GameHandler::new(|window| init_game_loop(app, window, on_engine_created));
    handler.run()
}

fn init_game_loop(
    app: Arc<dyn Application>,
    window: Window,
    on_engine_created: impl FnOnce(Arc<Engine>),
) -> Result<GameLoop> {
    let game_loop_config = app.game_loop_config();
    let engine = Engine::new(app, window)?;
    let game_loop = GameLoop::new(engine, game_loop_config)?;
    on_engine_created(game_loop.arc_engine());
    game_loop.engine().app().setup_scene()?;
    Ok(game_loop)
}
