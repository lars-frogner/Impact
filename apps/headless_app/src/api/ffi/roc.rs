//! FFI functions for Roc.

use crate::api;
use anyhow::{Context, anyhow};
use roc_platform_core::roc_std::{RocList, RocResult, RocStr};

#[unsafe(no_mangle)]
pub extern "C" fn roc_execute_engine_command(command_bytes: &RocList<u8>) -> RocResult<(), RocStr> {
    to_roc_result(
        api::execute_engine_command(command_bytes.as_slice())
            .context("Failed executing engine command"),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_create_entity_with_id(
    entity_id: u64,
    component_bytes: &RocList<u8>,
) -> RocResult<(), RocStr> {
    to_roc_result(
        api::create_entity_with_id(entity_id, component_bytes.as_slice())
            .with_context(|| format!("Failed creating entity with ID {entity_id}")),
    )
}

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

fn to_roc_result<T>(res: anyhow::Result<T>) -> RocResult<T, RocStr> {
    res.map_err(|error| anyhow!("{:#}", error).to_string().as_str().into())
        .into()
}
