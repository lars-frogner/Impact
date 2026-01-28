//! FFI for the app API specifically for calling from Roc.

use crate::interface::api;
use anyhow::{Context, anyhow};
use roc_platform_core::roc_std::{RocList, RocResult, RocStr};

#[unsafe(no_mangle)]
pub extern "C" fn roc_execute_ui_command(command_bytes: &RocList<u8>) -> RocResult<(), RocStr> {
    to_roc_result(
        api::execute_ui_command(command_bytes.as_slice()).context("Failed executing UI command"),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_execute_engine_command(command_bytes: &RocList<u8>) -> RocResult<(), RocStr> {
    to_roc_result(
        api::execute_engine_command(command_bytes.as_slice())
            .context("Failed executing engine command"),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_stage_entity_for_creation_with_id(
    entity_id: u64,
    component_bytes: &RocList<u8>,
) -> RocResult<(), RocStr> {
    to_roc_result(
        api::stage_entity_for_creation_with_id(entity_id, component_bytes.as_slice())
            .with_context(|| format!("Failed staging entity for creation with ID {entity_id}")),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_stage_entity_for_creation(
    component_bytes: &RocList<u8>,
) -> RocResult<(), RocStr> {
    to_roc_result(
        api::stage_entity_for_creation(component_bytes.as_slice())
            .context("Failed staging entity for creation"),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_stage_entities_for_creation(
    component_bytes: &RocList<u8>,
) -> RocResult<(), RocStr> {
    to_roc_result(
        api::stage_entities_for_creation(component_bytes.as_slice())
            .context("Failed staging multiple entities for creation"),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_stage_entity_for_update(
    entity_id: u64,
    component_bytes: &RocList<u8>,
) -> RocResult<(), RocStr> {
    to_roc_result(
        api::stage_entity_for_update(entity_id, component_bytes.as_slice())
            .context("Failed staging entity for update"),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_stage_entity_for_removal(entity_id: u64) -> RocResult<(), RocStr> {
    to_roc_result(
        api::stage_entity_for_removal(entity_id).context("Failed staging entity for removal"),
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

#[unsafe(no_mangle)]
pub extern "C" fn roc_update_entity(
    entity_id: u64,
    component_bytes: &RocList<u8>,
) -> RocResult<(), RocStr> {
    to_roc_result(
        api::update_entity(entity_id, component_bytes.as_slice())
            .with_context(|| format!("Failed updating entity with ID {entity_id}")),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_remove_entity(entity_id: u64) -> RocResult<(), RocStr> {
    to_roc_result(api::remove_entity(entity_id).context("Failed removing entity"))
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_read_entity_components(
    entity_id: u64,
    only_component_ids: &RocList<u64>,
) -> RocResult<RocList<u8>, RocStr> {
    let mut component_bytes = RocList::empty();
    to_roc_result(
        api::for_entity_components(entity_id, only_component_ids.as_slice(), &mut |component| {
            component_bytes.extend_from_slice(component);
        })
        .context("Failed reading entity components")
        .map(|_| component_bytes),
    )
}

fn to_roc_result<T>(res: anyhow::Result<T>) -> RocResult<T, RocStr> {
    res.map_err(|error| anyhow!("{:#}", error).to_string().as_str().into())
        .into()
}
