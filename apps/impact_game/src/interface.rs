//! Interfacing with the engine, scripts and external binaries.

pub mod api;
pub mod engine;
pub mod scripting;

use crate::Game;
use anyhow::Result;
use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard, RwLockWriteGuard,
};

type GameReadGuard = MappedRwLockReadGuard<'static, Game>;
type GameWriteGuard = MappedRwLockWriteGuard<'static, Game>;

static GAME: RwLock<Option<Game>> = RwLock::new(None);

fn access_game() -> GameReadGuard {
    RwLockReadGuard::map(GAME.read(), |game| {
        game.as_ref()
            .expect("Tried to access game before initialization")
    })
}

fn access_game_mut() -> GameWriteGuard {
    RwLockWriteGuard::map(GAME.write(), |game| {
        game.as_mut()
            .expect("Tried to access game before initialization")
    })
}

fn assert_game_not_accessed() {
    assert!(!GAME.is_locked());
}

fn with_dropped_write_guard(
    game: GameWriteGuard,
    f: impl FnOnce() -> Result<()>,
) -> Result<GameWriteGuard> {
    drop(game);
    f()?;
    Ok(access_game_mut())
}
