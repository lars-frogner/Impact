// Use the platform core to ensure it gets linked into this library
pub use roc_platform_core;

use roc_std::{RocList, RocResult, RocStr};

#[no_mangle]
pub extern "C" fn roc_fx_create_entity(_component_bytes: &RocList<u8>) -> RocResult<u64, RocStr> {
    dbg!("platform create_entity called");
    Ok(0).into()
}

#[no_mangle]
pub extern "C" fn roc_fx_create_entities(
    _component_bytes: &RocList<u8>,
) -> RocResult<RocList<u64>, RocStr> {
    dbg!("platform create_entity called");
    Ok(RocList::empty()).into()
}
