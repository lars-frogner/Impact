//! Loading of dynamic libraries.

#[macro_use]
pub mod macros;

#[cfg(feature = "hot_reloading")]
pub mod hot_reloading;

use std::{
    env, io,
    path::{Path, PathBuf},
};
use thiserror::Error;

pub type Library = libloading::Library;
pub type Symbol<'a, T> = libloading::Symbol<'a, T>;

pub type RwLock<T> = parking_lot::RwLock<T>;
pub type MappedRwLockReadGuard<'a, T> = parking_lot::MappedRwLockReadGuard<'a, T>;

pub type Result<T> = std::result::Result<T, LibraryError>;
pub type PathResult<T> = std::result::Result<T, PathError>;

pub trait LoadableLibrary: Sized {
    fn resolved_path() -> PathResult<PathBuf>;

    fn loaded_from_path(lib_path: &Path) -> Result<Self>;

    fn loaded() -> Result<Self> {
        let lib_path = Self::resolved_path()?;
        Self::loaded_from_path(&lib_path)
    }
}

pub trait DynamicLibrary: Sized {
    fn load() -> Result<()>;

    fn unload() -> Result<()>;

    fn replace(source_path: &Path, dest_path: &Path) -> Result<()>;
}

#[derive(Error, Debug)]
pub enum LibraryError {
    #[error("Failed to obtain library path")]
    PathError(#[from] PathError),

    #[error("Failed to load dynamic library at {path}")]
    LoadingLibrary {
        path: PathBuf,
        source: libloading::Error,
    },

    #[error("Failed to load symbol {symbol_name}")]
    LoadingSymbol {
        symbol_name: String,
        source: libloading::Error,
    },

    #[error("Failed to move library file from {source_path} to {dest_path}")]
    MovingLibrary {
        source_path: PathBuf,
        dest_path: PathBuf,
        source: io::Error,
    },

    #[error("Tried to load library when already loaded")]
    AlreadyLoaded,

    #[error("Tried to unload library when not loaded")]
    NotLoaded,
}

#[derive(Error, Debug)]
pub enum PathError {
    #[error("Failed to obtain executable path")]
    ExecutablePathNotFound { source: io::Error },

    #[error("Failed to resolve library path {path}")]
    PathResolution { path: PathBuf, source: io::Error },
}

/// Returns the path stored in the specified environment variable if set. If not
/// set, the given fallback path relative to the directory of the executable is
/// returned instead. The returned path is canonicalized.
///
/// # Errors
/// Returns an error if the path could not be canonicalized or the directory of
/// the executable could not be obtained.
pub fn resolve_path_from_env_with_fallback(
    path_env_var: &str,
    fallback_path: impl AsRef<Path>,
) -> PathResult<PathBuf> {
    let path = if let Ok(env_path) = env::var(path_env_var).map(PathBuf::from) {
        env_path
    } else {
        let fallback_path = fallback_path.as_ref();

        if fallback_path.is_absolute() {
            fallback_path.to_path_buf()
        } else {
            let executable_path = env::current_exe()
                .map_err(|source| PathError::ExecutablePathNotFound { source })?;

            let executable_dir = executable_path.parent().unwrap();

            executable_dir.join(fallback_path)
        }
    };

    path.canonicalize()
        .map_err(|source| PathError::PathResolution { path, source })
}

/// Only intended to be called from the `define_lib` macro.
pub fn __from_macro_load_library(lib_path: &Path) -> Result<Library> {
    log::debug!("Loading dynamic library at {}", lib_path.display());
    unsafe {
        Library::new(lib_path).map_err(|source| LibraryError::LoadingLibrary {
            path: lib_path.to_path_buf(),
            source,
        })
    }
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
            .map_err(|source| LibraryError::LoadingSymbol {
                symbol_name: symbol_name.to_string(),
                source,
            })
    }
}

/// Only intended to be called from the `define_lib` macro.
pub fn __from_macro_load<L: LoadableLibrary>(lib: &RwLock<Option<L>>) -> Result<()> {
    let mut lib_guard = lib.write();

    if lib_guard.is_some() {
        return Err(LibraryError::AlreadyLoaded);
    };

    *lib_guard = Some(L::loaded()?);

    Ok(())
}

/// Only intended to be called from the `define_lib` macro.
pub fn __from_macro_unload<L>(lib: &RwLock<Option<L>>) -> Result<()> {
    let mut lib_guard = lib.write();

    if lib_guard.is_none() {
        return Err(LibraryError::NotLoaded);
    }

    lib_guard.take();

    Ok(())
}

/// Only intended to be called from the `define_lib` macro.
pub fn __from_macro_replace<L: LoadableLibrary>(
    lib: &RwLock<Option<L>>,
    source_path: &Path,
    dest_path: &Path,
) -> Result<()> {
    let mut lib_guard = lib.write();

    if lib_guard.is_none() {
        return Err(LibraryError::NotLoaded);
    }

    // Unload before potentially overwriting the loaded library
    *lib_guard = None;

    std::fs::rename(source_path, dest_path).map_err(|source| LibraryError::MovingLibrary {
        source_path: source_path.to_path_buf(),
        dest_path: dest_path.to_path_buf(),
        source,
    })?;

    *lib_guard = Some(L::loaded_from_path(dest_path)?);

    Ok(())
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
            *lib_write_guard = Some(L::loaded()?);
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
