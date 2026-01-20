use roc_platform_core::roc_std::{RocList, RocResult, RocStr};

dynamic_lib::define_lib! {
    name = AppLib,
    path_env_var = "APP_LIB_PATH",
    fallback_path = "./libapp";

    unsafe fn roc_execute_game_command(command_bytes: &RocList<u8>) -> RocResult<(), RocStr>;
    unsafe fn roc_execute_ui_command(command_bytes: &RocList<u8>) -> RocResult<(), RocStr>;
    unsafe fn roc_execute_engine_command(command_bytes: &RocList<u8>) -> RocResult<(), RocStr>;
    unsafe fn roc_stage_entity_for_creation_with_id(entity_id: u64, component_bytes: &RocList<u8>) -> RocResult<(), RocStr>;
    unsafe fn roc_stage_entity_for_creation(component_bytes: &RocList<u8>) -> RocResult<(), RocStr>;
    unsafe fn roc_stage_entities_for_creation(component_bytes: &RocList<u8>) -> RocResult<(), RocStr>;
    unsafe fn roc_stage_entity_for_update(entity_id: u64, component_bytes: &RocList<u8>) -> RocResult<(), RocStr>;
    unsafe fn roc_stage_entity_for_removal(entity_id: u64) -> RocResult<(), RocStr>;
    unsafe fn roc_create_entity_with_id(entity_id: u64, component_bytes: &RocList<u8>) -> RocResult<(), RocStr>;
    unsafe fn roc_create_entity(component_bytes: &RocList<u8>) -> RocResult<u64, RocStr>;
    unsafe fn roc_create_entities(component_bytes: &RocList<u8>) -> RocResult<RocList<u64>, RocStr>;
    unsafe fn roc_update_entity(entity_id: u64, component_bytes: &RocList<u8>) -> RocResult<(), RocStr>;
    unsafe fn roc_remove_entity(entity_id: u64) -> RocResult<(), RocStr>;
    unsafe fn roc_read_entity_components(entity_id: u64, only_component_ids: &RocList<u64>) -> RocResult<RocList<u8>, RocStr>;
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_execute_game_command(
    command_bytes: &RocList<u8>,
) -> RocResult<(), RocStr> {
    load_and_then(|lib| unsafe { lib.roc_execute_game_command(command_bytes) })
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_execute_ui_command(command_bytes: &RocList<u8>) -> RocResult<(), RocStr> {
    load_and_then(|lib| unsafe { lib.roc_execute_ui_command(command_bytes) })
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_execute_engine_command(
    command_bytes: &RocList<u8>,
) -> RocResult<(), RocStr> {
    load_and_then(|lib| unsafe { lib.roc_execute_engine_command(command_bytes) })
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_stage_entity_for_creation_with_id(
    entity_id: u64,
    component_bytes: &RocList<u8>,
) -> RocResult<(), RocStr> {
    load_and_then(|lib| unsafe {
        lib.roc_stage_entity_for_creation_with_id(entity_id, component_bytes)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_stage_entity_for_creation(
    component_bytes: &RocList<u8>,
) -> RocResult<(), RocStr> {
    load_and_then(|lib| unsafe { lib.roc_stage_entity_for_creation(component_bytes) })
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_stage_entities_for_creation(
    component_bytes: &RocList<u8>,
) -> RocResult<(), RocStr> {
    load_and_then(|lib| unsafe { lib.roc_stage_entities_for_creation(component_bytes) })
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_stage_entity_for_update(
    entity_id: u64,
    component_bytes: &RocList<u8>,
) -> RocResult<(), RocStr> {
    load_and_then(|lib| unsafe { lib.roc_stage_entity_for_update(entity_id, component_bytes) })
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_stage_entity_for_removal(entity_id: u64) -> RocResult<(), RocStr> {
    load_and_then(|lib| unsafe { lib.roc_stage_entity_for_removal(entity_id) })
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_create_entity_with_id(
    entity_id: u64,
    component_bytes: &RocList<u8>,
) -> RocResult<(), RocStr> {
    load_and_then(|lib| unsafe { lib.roc_create_entity_with_id(entity_id, component_bytes) })
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_create_entity(component_bytes: &RocList<u8>) -> RocResult<u64, RocStr> {
    load_and_then(|lib| unsafe { lib.roc_create_entity(component_bytes) })
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_create_entities(
    component_bytes: &RocList<u8>,
) -> RocResult<RocList<u64>, RocStr> {
    load_and_then(|lib| unsafe { lib.roc_create_entities(component_bytes) })
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_update_entity(
    entity_id: u64,
    component_bytes: &RocList<u8>,
) -> RocResult<(), RocStr> {
    load_and_then(|lib| unsafe { lib.roc_update_entity(entity_id, component_bytes) })
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_remove_entity(entity_id: u64) -> RocResult<(), RocStr> {
    load_and_then(|lib| unsafe { lib.roc_remove_entity(entity_id) })
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_read_entity_components(
    entity_id: u64,
    only_component_ids: &RocList<u64>,
) -> RocResult<RocList<u8>, RocStr> {
    load_and_then(|lib| unsafe { lib.roc_read_entity_components(entity_id, only_component_ids) })
}

fn load_and_then<R>(call: impl FnOnce(&AppLib) -> RocResult<R, RocStr>) -> RocResult<R, RocStr> {
    match AppLib::load_and_acquire() {
        Ok(lib) => call(&lib),
        Err(err) => RocResult::err(RocStr::from(err.to_string().as_str())),
    }
}
