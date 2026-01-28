//! Interfacing with the engine, scripts and external binaries.

pub mod api;
pub mod engine;
pub mod scripting;

use crate::App;
use anyhow::Result;
use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard, RwLockWriteGuard,
};

type AppReadGuard = MappedRwLockReadGuard<'static, App>;
type AppWriteGuard = MappedRwLockWriteGuard<'static, App>;

static APP: RwLock<Option<App>> = RwLock::new(None);

fn access_app() -> AppReadGuard {
    RwLockReadGuard::map(APP.read(), |app| {
        app.as_ref()
            .expect("Tried to access app before initialization")
    })
}

fn access_app_mut() -> AppWriteGuard {
    RwLockWriteGuard::map(APP.write(), |app| {
        app.as_mut()
            .expect("Tried to access app before initialization")
    })
}

fn assert_app_not_accessed() {
    assert!(!APP.is_locked());
}

fn with_dropped_read_guard(
    app: AppReadGuard,
    f: impl FnOnce() -> Result<()>,
) -> Result<AppReadGuard> {
    drop(app);
    f()?;
    Ok(access_app())
}
