//! AmbientCG provider implementation.

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
    /// AmbientCG asset ID (e.g., "Grass001", "Bricks075")
    pub id: String,
    /// Texture resolution, defaults to 4K
    #[serde(default)]
    pub resolution: Resolution,
    /// Image format, defaults to JPG
    #[serde(default)]
    pub format: SurfaceAssetFormat,
}

/// Available texture resolutions on AmbientCG.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resolution {
    /// 1024x1024 resolution
    OneK,
    /// 2048x2048 resolution
    TwoK,
    /// 4096x4096 resolution (default)
    #[default]
    FourK,
    /// 8192x8192 resolution
    EightK,
}

/// Available image formats for surface materials.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurfaceAssetFormat {
    /// JPEG format (default, smaller file size)
    #[default]
    Jpg,
    /// PNG format (lossless, larger file size)
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
    /// Constructs the download URI for this AmbientCG asset.
    ///
    /// Creates a URL pointing to the ZIP archive containing all texture maps
    /// (color, normal, roughness, etc.) for the specified asset.
    pub fn obtain_fetch_uri(&self) -> String {
        match self {
            Self::Surface(info) => {
                format!(
                    "https://ambientcg.com/get?file={}_{}_{}.zip",
                    info.id,
                    info.resolution.as_str(),
                    info.format.as_str()
                )
            }
        }
    }
}
