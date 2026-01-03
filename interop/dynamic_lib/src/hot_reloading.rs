//! Hot reloading of dynamic libraries.

use crate::DynamicLibrary;
use notify::Watcher;
use parking_lot::RwLock;
use std::{
    env, io,
    marker::PhantomData,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Default)]
pub struct Reloader<L> {
    watcher: RwLock<Option<notify::RecommendedWatcher>>,
    _phantom: PhantomData<L>,
}

#[derive(Error, Debug)]
pub enum ReloadingError {
    #[error("Failed to create watcher")]
    CreatingWatcher { source: notify::Error },
    #[error("Failed to initiate watching of sources at {source_dir_path}")]
    InitiatingWatching {
        source_dir_path: PathBuf,
        source: notify::Error,
    },
    #[error("Failed to obtain executable path for relative source directory path")]
    ExecutablePathNotFound { source: io::Error },
}

pub type Result<T> = std::result::Result<T, ReloadingError>;

const CREATE_EVENT_KIND: notify::EventKind =
    notify::EventKind::Create(notify::event::CreateKind::File);

const MODIFY_EVENT_KIND: notify::EventKind = notify::EventKind::Modify(
    notify::event::ModifyKind::Data(notify::event::DataChange::Content),
);
const DELETE_EVENT_KIND: notify::EventKind =
    notify::EventKind::Remove(notify::event::RemoveKind::File);

impl<L: DynamicLibrary + 'static> Reloader<L> {
    pub const fn new() -> Self {
        Self {
            watcher: RwLock::new(None),
            _phantom: PhantomData,
        }
    }

    pub fn begin_watch(&self, source_dir_path: &Path) -> Result<()> {
        let source_dir_path = Self::resolve_source_dir_path(source_dir_path)?;

        let mut watcher = notify::recommended_watcher(Self::handle_event)
            .map_err(|source| ReloadingError::CreatingWatcher { source })?;

        watcher
            .watch(&source_dir_path, notify::RecursiveMode::Recursive)
            .map_err(|source| ReloadingError::InitiatingWatching {
                source_dir_path,
                source,
            })?;

        *self.watcher.write() = Some(watcher);

        Ok(())
    }

    fn resolve_source_dir_path(source_dir_path: &Path) -> Result<PathBuf> {
        let source_dir_path = if source_dir_path.is_absolute() {
            source_dir_path.to_path_buf()
        } else {
            let executable_path = env::current_exe()
                .map_err(|source| ReloadingError::ExecutablePathNotFound { source })?;

            let executable_dir = executable_path.parent().unwrap();

            executable_dir.join(source_dir_path)
        };
    }

    fn handle_event(event: notify::Result<notify::Event>) {
        let event = match event {
            Ok(event) => event,
            Err(err) => {
                log::error!("Watching failed with error: {err}");
                return;
            }
        };
        dbg!(&event);
        if matches!(
            event.kind,
            CREATE_EVENT_KIND | MODIFY_EVENT_KIND | DELETE_EVENT_KIND
        ) {}
    }
}
