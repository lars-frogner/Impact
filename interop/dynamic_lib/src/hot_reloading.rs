//! Hot reloading of dynamic libraries.

use crate::DynamicLibrary;
use crossbeam_channel::{Receiver, Sender, select};
use notify::Watcher as _;
use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
    process::Command,
    thread::JoinHandle,
};
use thiserror::Error;

/// Handles rebuilding and reloading of a dynamic library when its source code
/// changes.
#[derive(Debug)]
pub struct LibraryReloader<L> {
    watcher_command_sender: Sender<WatcherCommand>,
    reloader_command_sender: Sender<ReloaderCommand>,
    reload_completed_receiver: Receiver<()>,
    watcher_join_handle: Option<JoinHandle<()>>,
    reloader_join_handle: Option<JoinHandle<()>>,
    _watcher: notify::RecommendedWatcher,
    _phantom: PhantomData<L>,
}

#[derive(Error, Debug)]
pub enum HotReloadingError {
    #[error("Failed to create watcher")]
    CreatingWatcher { source: notify::Error },

    #[error("Failed to initiate watching of sources at {source_dir_path}")]
    InitiatingWatching {
        source_dir_path: PathBuf,
        source: notify::Error,
    },
}

pub type Result<T> = std::result::Result<T, HotReloadingError>;

enum WatcherCommand {
    Resume,
    Exit,
}

enum ReloaderCommand {
    Reload,
    Exit,
}

const EVENT_CHANNEL_CAPACITY: usize = 1000;
const COMMAND_CHANNEL_CAPACITY: usize = 128;

impl<L: DynamicLibrary> LibraryReloader<L> {
    /// Creates a reloader for the library of type `L`. The given source code
    /// directory will be watched for changes. When a change is detected, the
    /// given build command is used to rebuild the dynamic library from the
    /// updated source code. If the build succeeds, the rebuilt library, which
    /// is expected to be stored at the specified output path, is moved to a
    /// different adjacent path (to prevent later builds from overwriting the
    /// loaded library) and loaded.
    ///
    /// This method spawns two threads, a watcher thread for monitoring source
    /// code and a reloader thread for rebuilding and loading the new library.
    /// Both threads are joined when this object drops.
    pub fn new(
        source_dir_path: PathBuf,
        build_command: Command,
        build_output_lib_path: PathBuf,
    ) -> Result<Self> {
        let (watcher_command_sender, watcher_command_receiver) =
            crossbeam_channel::bounded(COMMAND_CHANNEL_CAPACITY);

        let (reloader_command_sender, reloader_command_receiver) =
            crossbeam_channel::bounded(COMMAND_CHANNEL_CAPACITY);

        let (reload_completed_sender, reload_completed_receiver) = crossbeam_channel::bounded(1);

        let (watcher, watcher_join_handle) = spawn_watcher(
            source_dir_path,
            watcher_command_receiver,
            reloader_command_sender.clone(),
        )?;

        let reloader_join_handle = spawn_reloader::<L>(
            build_command,
            build_output_lib_path,
            reloader_command_receiver,
            watcher_command_sender.clone(),
            reload_completed_sender,
        );

        Ok(Self {
            watcher_command_sender,
            reloader_command_sender,
            reload_completed_receiver,
            watcher_join_handle: Some(watcher_join_handle),
            reloader_join_handle: Some(reloader_join_handle),
            _watcher: watcher,
            _phantom: PhantomData,
        })
    }

    /// Whether the dynamic library has been reloaded since the last time this
    /// method was called.
    pub fn reloaded_since_last_check(&self) -> bool {
        self.reload_completed_receiver.try_recv().is_ok()
    }
}

impl<L> Drop for LibraryReloader<L> {
    fn drop(&mut self) {
        // Instruct the worker threads to exit and then wait for them to finish.
        // We ignore errors at this point.

        let _ = self.watcher_command_sender.send(WatcherCommand::Exit);
        let _ = self.reloader_command_sender.send(ReloaderCommand::Exit);

        if let Some(handle) = self.watcher_join_handle.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.reloader_join_handle.take() {
            let _ = handle.join();
        }
    }
}

fn spawn_watcher(
    source_dir_path: PathBuf,
    watcher_command_receiver: Receiver<WatcherCommand>,
    reloader_command_sender: Sender<ReloaderCommand>,
) -> Result<(notify::RecommendedWatcher, JoinHandle<()>)> {
    let (event_sender, event_receiver) = crossbeam_channel::bounded(EVENT_CHANNEL_CAPACITY);

    let mut watcher = notify::recommended_watcher(event_sender)
        .map_err(|source| HotReloadingError::CreatingWatcher { source })?;

    log::info!(
        "Watching for source code changes in {}",
        source_dir_path.display()
    );
    watcher
        .watch(&source_dir_path, notify::RecursiveMode::Recursive)
        .map_err(|source| HotReloadingError::InitiatingWatching {
            source_dir_path,
            source,
        })?;

    let join_handle = std::thread::spawn(move || {
        let mut ignore_events = false;
        loop {
            select! {
                recv(event_receiver) -> event => {
                    let Ok(event) = event else {
                        log::debug!("Source code watcher event channel closed");
                        break;
                    };
                    if ignore_events {
                        continue;
                    }
                    let event = match event {
                        Ok(event) => event,
                        Err(err) => {
                            log::error!("Watching failed with error: {err}");
                            continue;
                        }
                    };
                    if event_caused_source_code_change(event.kind) {
                        log::debug!("Watcher received source code change event: {:?}", event.kind);

                        if reloader_command_sender.send(ReloaderCommand::Reload).is_err() {
                            // All receivers dropped, nothing more to do
                            break;
                        }
                        // Ignore events until we are told to resume
                        ignore_events = true;
                    }
                }
                recv(watcher_command_receiver) -> command => {
                    let Ok(command) = command else {
                        // All senders dropped, time to exit
                        break;
                    };
                    match command {
                        WatcherCommand::Resume => {
                            log::debug!("Source code watcher received resume command");
                            ignore_events = false;
                        }
                        WatcherCommand::Exit => {
                            log::debug!("Source code watcher received exit command");
                            break;
                        }
                    }
                }
            }
        }
        log::info!("Shutting down source code watcher");
    });

    Ok((watcher, join_handle))
}

fn event_caused_source_code_change(event_kind: notify::EventKind) -> bool {
    use notify::{
        EventKind,
        event::{CreateKind, DataChange, ModifyKind, RemoveKind},
    };
    matches!(
        event_kind,
        EventKind::Create(CreateKind::File)
            | EventKind::Remove(RemoveKind::File | RemoveKind::Folder)
            | EventKind::Modify(
                ModifyKind::Data(DataChange::Content | DataChange::Any) | ModifyKind::Name(_)
            )
    )
}

fn spawn_reloader<L: DynamicLibrary>(
    mut build_command: Command,
    build_output_lib_path: PathBuf,
    reloader_command_receiver: Receiver<ReloaderCommand>,
    watcher_command_sender: Sender<WatcherCommand>,
    reload_completed_sender: Sender<()>,
) -> JoinHandle<()> {
    log::debug!(
        "Library reloader will use build program `{}` in {} outputting to {}",
        build_command.get_program().display(),
        build_command
            .get_current_dir()
            .unwrap_or_else(|| Path::new("<cwd>"))
            .display(),
        build_output_lib_path.display(),
    );

    let loaded_lib_path = build_output_lib_path.with_added_extension("loaded");

    std::thread::spawn(move || {
        for command in reloader_command_receiver {
            match command {
                ReloaderCommand::Reload => {
                    log::debug!("Library reloader received reload command");

                    match build_command.output() {
                        Ok(output) => {
                            if log::log_enabled!(log::Level::Debug) {
                                log::debug!("Build command completed with output:");
                                for line in String::from_utf8_lossy(&output.stdout).lines() {
                                    log::debug!("[stdout] {}", line);
                                }
                                for line in String::from_utf8_lossy(&output.stderr).lines() {
                                    log::debug!("[stderr] {}", line);
                                }
                            }
                            if output.status.success() {
                                match L::replace(&build_output_lib_path, &loaded_lib_path) {
                                    Ok(()) => {
                                        log::info!("Successfully replaced dynamic library");

                                        // Notify about the completed reload by
                                        // sending a unit value if the channel
                                        // is empty
                                        let _ = reload_completed_sender.try_send(());
                                    }
                                    Err(err) => {
                                        log::error!("Reloader failed to replace library: {err}");
                                    }
                                }
                            } else {
                                log::warn!(
                                    "Build command from library reloader finished with errors"
                                );
                            }
                        }
                        Err(err) => {
                            log::error!("Library reloader failed to run build command: {err}");
                        }
                    }
                    if watcher_command_sender.send(WatcherCommand::Resume).is_err() {
                        // Watcher has exited, so we should as well
                        break;
                    }
                }
                ReloaderCommand::Exit => {
                    log::debug!("Library reloader received exit command");
                    break;
                }
            }
        }
        log::info!("Shutting down library reloader");
    })
}
