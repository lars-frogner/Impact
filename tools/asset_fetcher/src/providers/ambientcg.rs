//! AmbientCG provider implementation.

use crate::providers::AssetDownload;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Asset information specific to AmbientCG provider.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetInfo {
    /// Surface material (textures for PBR rendering)
    Surface(SurfaceAssetInfo),
}

/// Information required to fetch a surface material from AmbientCG.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceAssetInfo {
    /// AmbientCG asset ID (e.g., "Grass001", "Bricks075").
    pub id: String,
    /// Texture resolution, defaults to 4K.
    #[serde(default)]
    pub resolution: Resolution,
    /// Image format, defaults to JPG.
    #[serde(default)]
    pub format: SurfaceAssetFormat,
}

/// Available texture resolutions on AmbientCG.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resolution {
    /// 1024x1024 resolution.
    OneK,
    /// 2048x2048 resolution.
    TwoK,
    /// 4096x4096 resolution (default).
    #[default]
    FourK,
    /// 8192x8192 resolution.
    EightK,
}

/// Available image formats for surface materials.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurfaceAssetFormat {
    /// JPEG format (default, smaller file size).
    #[default]
    Jpg,
    /// PNG format (lossless, larger file size).
    Png,
}

impl Resolution {
    fn as_str(&self) -> &'static str {
        match self {
            Self::OneK => "1K",
            Self::TwoK => "2K",
            Self::FourK => "4K",
            Self::EightK => "8K",
        }
    }
}

impl SurfaceAssetFormat {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Jpg => "jpg",
            Self::Png => "png",
        }
    }
}

impl AssetInfo {
    /// Gets all downloads required for this AmbientCG asset.
    ///
    /// AmbientCG provides assets as ZIP archives containing all texture maps,
    /// so this always returns a single download.
    pub fn get_downloads(&self) -> Result<Vec<AssetDownload>> {
        match self {
            Self::Surface(info) => {
                let url = format!(
                    "https://ambientcg.com/get?file={}_{}_{}.zip",
                    info.id,
                    info.resolution.as_str(),
                    info.format.as_str()
                );

                Ok(vec![AssetDownload {
                    url,
                    file_path: format!("{}_{}.zip", info.id, info.resolution.as_str()),
                    size: None,
                    md5: None,
                }])
            }
        }
    }
}
