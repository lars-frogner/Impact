//! Calling functions in a Roc script.

use crate::testing::TestScene;
use anyhow::{Context, Result, anyhow};
use ffi_helpers::define_ffi;
use impact::roc_integration::Roc;
use roc_platform_core::roc_std::{RocList, RocResult, RocStr};

define_ffi! {
    name = ScriptFFI,
    lib_path_env = "SCRIPT_LIB_PATH",
    lib_path_default = "./libscript",
    roc__setup_scene_extern_1_exposed => unsafe extern "C" fn(RocList<u8>) -> RocResult<(), RocStr>,
}

pub fn setup_scene(scene: TestScene) -> Result<()> {
    let mut bytes = RocList::from_slice(&[0; TestScene::SERIALIZED_SIZE]);
    scene.write_roc_bytes(bytes.as_mut_slice())?;

    ScriptFFI::call(
        |ffi| from_roc_result(unsafe { (ffi.roc__setup_scene_extern_1_exposed)(bytes) }),
        |error| Err(anyhow!("{error:#}")),
    )
    .with_context(|| format!("Failed setup for scene {scene}"))
}

fn from_roc_result<T>(res: RocResult<T, RocStr>) -> Result<T> {
    Result::from(res).map_err(|error| anyhow!("{error}"))
}
