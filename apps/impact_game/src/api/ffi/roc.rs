//! FFI functions for Roc.

use crate::api;
use anyhow::{Context, anyhow};
use roc_platform_core::roc_std::{RocList, RocResult, RocStr};

#[unsafe(no_mangle)]
pub extern "C" fn roc_create_entity(component_bytes: &RocList<u8>) -> RocResult<u64, RocStr> {
    to_roc_result(api::create_entity(component_bytes.as_slice()).context("Failed creating entity"))
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_create_entities(
    component_bytes: &RocList<u8>,
) -> RocResult<RocList<u64>, RocStr> {
    to_roc_result(
        api::create_entities(component_bytes.as_slice())
            .map(RocList::from_iter)
            .context("Failed creating multiple entities"),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_set_skybox(
    cubemap_texture_name: &RocStr,
    max_luminance: f32,
) -> RocResult<(), RocStr> {
    to_roc_result(
        api::set_skybox(cubemap_texture_name.as_str(), max_luminance)
            .with_context(|| format!("Failed setting skybox to {cubemap_texture_name}")),
    )
}

fn to_roc_result<T>(res: anyhow::Result<T>) -> RocResult<T, RocStr> {
    res.map_err(|error| anyhow!("{:#}", error).to_string().as_str().into())
        .into()
}
