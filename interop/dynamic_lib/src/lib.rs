//! Loading of dynamic libraries.

#[macro_use]
pub mod macros;

use std::{
    env, io,
    path::{Path, PathBuf},
};
use thiserror::Error;

pub type Library = libloading::Library;
pub type Symbol<'a, T> = libloading::Symbol<'a, T>;

pub type RwLock<T> = parking_lot::RwLock<T>;
pub type MappedRwLockReadGuard<'a, T> = parking_lot::MappedRwLockReadGuard<'a, T>;

pub type Result<T> = std::result::Result<T, LoadingError>;

pub trait LoadableLibrary: Sized {
    fn new_loaded() -> Result<Self>;
}

pub trait DynamicLibrary: Sized {
    fn load() -> Result<()>;

    fn unload() -> Result<()>;
}

#[derive(Error, Debug)]
pub enum LoadingError {
    #[error("Failed to obtain executable path for default library path {default_path}")]
    ExecutablePathNotFound {
        default_path: String,
        source: io::Error,
    },

    #[error("Failed to resolve library path {path}")]
    LibraryPathResolution { path: String, source: io::Error },

    #[error("Failed to load dynamic library at {path}")]
    LibraryLoading {
        path: String,
        source: libloading::Error,
    },

    #[error("Failed to load symbol {symbol_name}")]
    SymbolLoading {
        symbol_name: String,
        source: libloading::Error,
    },

    #[error("Tried to load library when already loaded")]
    AlreadyLoaded,

    #[error("Tried to unload library when not loaded")]
    NotLoaded,
}

/// Only intended to be called from the `define_lib` macro.
pub fn __from_macro_load<L: LoadableLibrary>(lib: &RwLock<Option<L>>) -> Result<()> {
    let mut lib_guard = lib.write();

    if lib_guard.is_some() {
        return Err(LoadingError::AlreadyLoaded);
    };

    *lib_guard = Some(L::new_loaded()?);

    Ok(())
}

/// Only intended to be called from the `define_lib` macro.
pub fn __from_macro_unload<L>(lib: &RwLock<Option<L>>) -> Result<()> {
    let mut lib_guard = lib.write();

    if lib_guard.is_none() {
        return Err(LoadingError::NotLoaded);
    }

    lib_guard.take();

    Ok(())
}

/// Only intended to be called from the `define_lib` macro.
pub fn __from_macro_load_library(lib_path_env: &str, lib_path_default: &str) -> Result<Library> {
    let path = get_library_path(lib_path_env, lib_path_default)?;
    load_library(&path)
}

/// Only intended to be called from the `define_lib` macro.
pub fn __from_macro_load_symbol<'a, T>(
    library: &'a Library,
    symbol_name: &str,
) -> Result<Symbol<'a, T>> {
    log::debug!("Loading symbol {symbol_name}");
    unsafe {
        library
            .get::<T>(symbol_name)
            .map_err(|source| LoadingError::SymbolLoading {
                symbol_name: symbol_name.to_string(),
                source,
            })
    }
}

/// Only intended to be called from the `define_lib` macro.
#[inline]
pub fn __from_macro_acquire<L>(lib: &RwLock<Option<L>>) -> MappedRwLockReadGuard<'_, L> {
    parking_lot::RwLockReadGuard::map(lib.read(), |lib| {
        lib.as_ref()
            .expect("Tried to call symbol in unloaded library")
    })
}

/// Only intended to be called from the `define_lib` macro.
#[inline]
pub fn __from_macro_load_and_acquire<L: LoadableLibrary>(
    lib: &RwLock<Option<L>>,
) -> Result<MappedRwLockReadGuard<'_, L>> {
    let lib_read_guard = lib.read();

    if lib_read_guard.is_none() {
        // If the library is not loaded, acquire write access to it. We could
        // have used `upgradable_read` instead of `read` to avoid a gap between
        // dropping the read lock and acquiring the write lock, but that would
        // block threads unnecessarily when the library is loaded.
        drop(lib_read_guard);
        let mut lib_write_guard = lib.write();

        // Since another thread could have loaded the library before we acquired
        // the write lock, we must check again whether it is loaded
        if lib_write_guard.is_none() {
            *lib_write_guard = Some(L::new_loaded()?);
        }

        let lib_read_guard = parking_lot::RwLockWriteGuard::downgrade(lib_write_guard);

        // Downgrading to a read lock is atomic, so we know the library must
        // still be loaded
        Ok(parking_lot::RwLockReadGuard::map(lib_read_guard, |lib| {
            lib.as_ref().unwrap()
        }))
    } else {
        Ok(parking_lot::RwLockReadGuard::map(lib_read_guard, |lib| {
            lib.as_ref().unwrap()
        }))
    }
}

fn get_library_path(lib_path_env: &str, lib_path_default: &str) -> Result<PathBuf> {
    let library_path = match env::var(lib_path_env).map(PathBuf::from) {
        Ok(lib_path) => lib_path,
        Err(_) => env::current_exe()
            .map_err(|source| LoadingError::ExecutablePathNotFound {
                default_path: lib_path_default.to_string(),
                source,
            })?
            .parent()
            .unwrap()
            .join(lib_path_default),
    };

    library_path
        .canonicalize()
        .map_err(|source| LoadingError::LibraryPathResolution {
            path: library_path.display().to_string(),
            source,
        })
}

fn load_library(library_path: &Path) -> Result<Library> {
    log::debug!("Loading dynamic library at {}", library_path.display());
    unsafe {
        Library::new(library_path).map_err(|source| LoadingError::LibraryLoading {
            path: library_path.display().to_string(),
            source,
        })
    }
}
