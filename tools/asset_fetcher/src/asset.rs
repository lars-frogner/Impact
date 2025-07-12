//! Asset specification.

use crate::providers::AssetInfo;
use serde::{Deserialize, Serialize};

/// An asset to be fetched from a provider.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asset {
    /// User-specified identifier for the asset.
    pub name: String,
    /// Provider-dependent info about the asset.
    pub info: AssetInfo,
}

/// A collection of assets to be processed together.
pub type AssetList = Vec<Asset>;
