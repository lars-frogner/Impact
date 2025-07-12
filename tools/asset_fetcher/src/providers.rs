//! Asset providers.

pub mod ambientcg;

use serde::{Deserialize, Serialize};

/// Provider-specific asset information.
///
/// This enum contains the asset metadata needed by each supported provider to
/// construct download URLs and handle asset-specific requirements.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetInfo {
    /// Asset information for AmbientCG provider
    AmbientCG(ambientcg::AssetInfo),
}

impl AssetInfo {
    /// Constructs the download URI for this asset based on provider-specific
    /// information.
    ///
    /// Each provider implements its own URI construction logic based on
    /// the asset metadata (ID, resolution, format, etc.).
    pub fn obtain_fetch_uri(&self) -> String {
        match self {
            Self::AmbientCG(info) => info.obtain_fetch_uri(),
        }
    }
}
