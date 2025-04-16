//!

use anyhow::{Context, Result, anyhow};
use ffi_utils::define_ffi;
use roc_platform_core::roc_std::{RocResult, RocStr};

define_ffi! {
    name = ScriptFFI,
    lib_path_env = "SCRIPT_LIB_PATH",
    lib_path_default = "../../../lib/libscript",
    roc__setup_scene_for_host_1_exposed => unsafe extern "C" fn(i32) -> RocResult<(), RocStr>,
}

pub fn setup_scene() -> Result<()> {
    ScriptFFI::call(
        |ffi| from_roc_result(unsafe { (ffi.roc__setup_scene_for_host_1_exposed)(0) }),
        |error| Err(anyhow!("{:#}", error)),
    )
    .with_context(|| "Failed scene setup")
}

fn from_roc_result<T>(res: RocResult<T, RocStr>) -> Result<T> {
    Result::from(res).map_err(|error| anyhow!("{error}"))
}
