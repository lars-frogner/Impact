use anyhow::anyhow;
use ffi_utils::define_ffi;
use roc_platform_core::roc_std::{RocList, RocResult, RocStr};

define_ffi! {
    name = AppFFI,
    lib_path_env = "APP_LIB_PATH",
    lib_path_default = "./libapp",
    roc_execute_engine_command => unsafe extern "C" fn(&RocList<u8>) -> RocResult<(), RocStr>,
    roc_stage_entity_for_creation_with_id => unsafe extern "C" fn(u64, &RocList<u8>) -> RocResult<(), RocStr>,
    roc_stage_entity_for_creation => unsafe extern "C" fn(&RocList<u8>) -> RocResult<(), RocStr>,
    roc_stage_entities_for_creation => unsafe extern "C" fn(&RocList<u8>) -> RocResult<(), RocStr>,
    roc_stage_entity_for_update => unsafe extern "C" fn(u64, &RocList<u8>) -> RocResult<(), RocStr>,
    roc_stage_entity_for_removal => unsafe extern "C" fn(u64) -> RocResult<(), RocStr>,
    roc_create_entity_with_id => unsafe extern "C" fn(u64, &RocList<u8>) -> RocResult<(), RocStr>,
    roc_create_entity => unsafe extern "C" fn(&RocList<u8>) -> RocResult<u64, RocStr>,
    roc_create_entities => unsafe extern "C" fn(&RocList<u8>) -> RocResult<RocList<u64>, RocStr>,
    roc_update_entity => unsafe extern "C" fn(u64, &RocList<u8>) -> RocResult<(), RocStr>,
    roc_remove_entity => unsafe extern "C" fn(u64) -> RocResult<(), RocStr>,
    roc_read_entity_components => unsafe extern "C" fn(u64, &RocList<u64>) -> RocResult<RocList<u8>, RocStr>,
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_execute_engine_command(
    command_bytes: &RocList<u8>,
) -> RocResult<(), RocStr> {
    AppFFI::call(
        |ffi| unsafe { (ffi.roc_execute_engine_command)(command_bytes) },
        to_roc_err,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_stage_entity_for_creation_with_id(
    entity_id: u64,
    component_bytes: &RocList<u8>,
) -> RocResult<(), RocStr> {
    AppFFI::call(
        |ffi| unsafe { (ffi.roc_stage_entity_for_creation_with_id)(entity_id, component_bytes) },
        to_roc_err,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_stage_entity_for_creation(
    component_bytes: &RocList<u8>,
) -> RocResult<(), RocStr> {
    AppFFI::call(
        |ffi| unsafe { (ffi.roc_stage_entity_for_creation)(component_bytes) },
        to_roc_err,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_stage_entities_for_creation(
    component_bytes: &RocList<u8>,
) -> RocResult<(), RocStr> {
    AppFFI::call(
        |ffi| unsafe { (ffi.roc_stage_entities_for_creation)(component_bytes) },
        to_roc_err,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_stage_entity_for_update(
    entity_id: u64,
    component_bytes: &RocList<u8>,
) -> RocResult<(), RocStr> {
    impact_log::trace!("Platform: stage_entity_for_update called");
    AppFFI::call(
        |ffi| unsafe { (ffi.roc_stage_entity_for_update)(entity_id, component_bytes) },
        to_roc_err,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_stage_entity_for_removal(entity_id: u64) -> RocResult<(), RocStr> {
    AppFFI::call(
        |ffi| unsafe { (ffi.roc_stage_entity_for_removal)(entity_id) },
        to_roc_err,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_create_entity_with_id(
    entity_id: u64,
    component_bytes: &RocList<u8>,
) -> RocResult<(), RocStr> {
    AppFFI::call(
        |ffi| unsafe { (ffi.roc_create_entity_with_id)(entity_id, component_bytes) },
        to_roc_err,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_create_entity(component_bytes: &RocList<u8>) -> RocResult<u64, RocStr> {
    AppFFI::call(
        |ffi| unsafe { (ffi.roc_create_entity)(component_bytes) },
        to_roc_err,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_create_entities(
    component_bytes: &RocList<u8>,
) -> RocResult<RocList<u64>, RocStr> {
    AppFFI::call(
        |ffi| unsafe { (ffi.roc_create_entities)(component_bytes) },
        to_roc_err,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_update_entity(
    entity_id: u64,
    component_bytes: &RocList<u8>,
) -> RocResult<(), RocStr> {
    impact_log::trace!("Platform: update_entity called");
    AppFFI::call(
        |ffi| unsafe { (ffi.roc_update_entity)(entity_id, component_bytes) },
        to_roc_err,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_remove_entity(entity_id: u64) -> RocResult<(), RocStr> {
    AppFFI::call(
        |ffi| unsafe { (ffi.roc_remove_entity)(entity_id) },
        to_roc_err,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_read_entity_components(
    entity_id: u64,
    only_component_ids: &RocList<u64>,
) -> RocResult<RocList<u8>, RocStr> {
    impact_log::trace!("Platform: read_entity_components called");
    AppFFI::call(
        |ffi| unsafe { (ffi.roc_read_entity_components)(entity_id, only_component_ids) },
        to_roc_err,
    )
}

fn to_roc_err<T>(error: &anyhow::Error) -> RocResult<T, RocStr> {
    RocResult::err(RocStr::from(anyhow!("{:#}", error).to_string().as_str()))
}
