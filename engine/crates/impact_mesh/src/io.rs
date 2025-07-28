//! Input/output of mesh data.

#[cfg(feature = "obj")]
pub mod obj;
#[cfg(feature = "ply")]
pub mod ply;

use anyhow::{Result, anyhow, bail};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriangleMeshFileFormat {
    Obj,
    Ply,
}

impl TriangleMeshFileFormat {
    /// Tries to determine the triangle mesh file format of the given path.
    pub fn from_path(file_path: &Path) -> Result<Self> {
        let Some(extension) = file_path.extension() else {
            bail!(
                "Missing extension for triangle mesh file {}",
                file_path.display()
            );
        };
        match &*extension.to_string_lossy().to_lowercase() {
            "obj" => Ok(Self::Obj),
            "ply" => Ok(Self::Ply),
            other => Err(anyhow!(
                "Unsupported triangle mesh file format {other} for triangle mesh file {}",
                file_path.display()
            )),
        }
    }
}
