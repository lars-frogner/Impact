//! CGBookcase provider implementation.

use crate::providers::AssetDownload;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Asset information specific to CGBookcase provider.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetInfo {
    /// CGBookcase ID (e.g., "Soapstone01").
    pub id: String,
    /// Texture resolution, defaults to 4K.
    #[serde(default)]
    pub resolution: Resolution,
}

/// Available texture resolutions on CGBookcase.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resolution {
    OneK,
    TwoK,
    ThreeK,
    #[default]
    FourK,
}

impl Resolution {
    fn as_str(&self) -> &'static str {
        match self {
            Self::OneK => "1K",
            Self::TwoK => "2K",
            Self::ThreeK => "3K",
            Self::FourK => "4K",
        }
    }
}

impl AssetInfo {
    /// Gets all downloads required for this CGBookcase asset.
    ///
    /// CGBookcase provides assets as ZIP archives containing all texture maps,
    /// so this always returns a single download.
    pub fn get_downloads(&self) -> Result<Vec<AssetDownload>> {
        let url = format!(
            "https://www.cgbookcase.com/textures/thanks?t={}_MR_{}.zip",
            self.id,
            self.resolution.as_str(),
        );

        Ok(vec![AssetDownload {
            url,
            file_path: format!("{}_MR_{}.zip", self.id, self.resolution.as_str()),
            size: None,
            md5: None,
        }])
    }
}
