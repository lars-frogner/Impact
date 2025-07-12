//! Asset providers.

pub mod ambientcg;
pub mod cgbookcase;
pub mod polyhaven;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Provider-specific asset information.
///
/// This enum contains the asset metadata needed by each supported provider to
/// obtain download URLs and handle asset-specific requirements.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetInfo {
    /// Asset information for AmbientCG provider.
    AmbientCG(ambientcg::AssetInfo),
    /// Asset information for CGBookcase provider.
    CGBookcase(cgbookcase::AssetInfo),
    /// Asset information for Poly Haven provider.
    PolyHaven(polyhaven::AssetInfo),
}

/// Represents a single file download within an asset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetDownload {
    /// The download URL for this file.
    pub url: String,
    /// The relative path where this file should be saved within the asset directory.
    pub file_path: String,
    /// Optional expected file size in bytes.
    pub size: Option<u64>,
    /// Optional MD5 hash for verification.
    pub md5: Option<String>,
}

impl AssetInfo {
    /// Gets all downloads required for this asset.
    ///
    /// For simple assets (like AmbientCG), this returns a single download.
    /// For complex assets (like Poly Haven), this may return multiple downloads
    /// for different content types (diffuse, normal, rough, etc.).
    pub fn get_downloads(&self) -> Result<Vec<AssetDownload>> {
        match self {
            Self::AmbientCG(info) => info.get_downloads(),
            Self::CGBookcase(info) => info.get_downloads(),
            Self::PolyHaven(info) => info.get_downloads(),
        }
    }
}
