//! Calling functions in a Roc script.

use crate::{interface::assert_app_not_accessed, testing::TestScene};
use anyhow::{Context, Result, anyhow};
use impact::roc_integration::Roc;
use roc_platform_core::roc_std::{RocList, RocResult, RocStr};

dynamic_lib::define_lib! {
    name = ScriptLib,
    path_env_var = "SCRIPT_LIB_PATH",
    fallback_path = "./libscript";

    unsafe fn roc__setup_scene_extern_1_exposed(scene_bytes: RocList<u8>) -> RocResult<(), RocStr>;
}

pub(crate) fn setup_scene(scene: TestScene) -> Result<()> {
    // Ensure no lock is being held on the app in case the script calls back
    // into our API
    assert_app_not_accessed();

    let mut scene_bytes = RocList::from_slice(&[0; TestScene::SERIALIZED_SIZE]);
    scene.write_roc_bytes(scene_bytes.as_mut_slice())?;

    from_roc_result(unsafe { ScriptLib::acquire().roc__setup_scene_extern_1_exposed(scene_bytes) })
        .with_context(|| format!("Failed setup for scene {scene}"))
}

fn from_roc_result<T>(res: RocResult<T, RocStr>) -> Result<T> {
    Result::from(res).map_err(|error| anyhow!("{error}"))
}
