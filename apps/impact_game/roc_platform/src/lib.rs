use anyhow::anyhow;
use ffi_utils::define_ffi;
use roc_platform_core::roc_std::{RocList, RocResult, RocStr};

define_ffi! {
    name = ImpactGameFFI,
    lib_path_env = "IMPACT_GAME_LIB_PATH",
    lib_path_default = "../../../lib/libapp",
    roc_create_entity => unsafe extern "C" fn(&RocList<u8>) -> RocResult<u64, RocStr>,
    roc_create_entities => unsafe extern "C" fn(&RocList<u8>) -> RocResult<RocList<u64>, RocStr>,
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_create_entity(component_bytes: &RocList<u8>) -> RocResult<u64, RocStr> {
    println!("Platform: create_entity called");
    log::debug!("Platform: create_entity called");
    ImpactGameFFI::call(
        |ffi| unsafe { (ffi.roc_create_entity)(component_bytes) },
        to_roc_err,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn roc_fx_create_entities(
    component_bytes: &RocList<u8>,
) -> RocResult<RocList<u64>, RocStr> {
    println!("Platform: create_entities called");
    log::debug!("Platform: create_entities called");
    ImpactGameFFI::call(
        |ffi| unsafe { (ffi.roc_create_entities)(component_bytes) },
        to_roc_err,
    )
}

fn to_roc_err<T>(error: &anyhow::Error) -> RocResult<T, RocStr> {
    RocResult::err(RocStr::from(anyhow!("{:#}", error).to_string().as_str()))
}
