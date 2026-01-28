//! Calling functions in a Roc script.

pub mod hot_reloading;

use crate::{
    input::InputContext, interface::assert_game_not_accessed, setup::SetupContext,
    update::UpdateContext,
};
use anyhow::{Context, Result, anyhow};
use impact::{
    input::{
        key::KeyboardEvent,
        mouse::{MouseButtonEvent, MouseDragEvent, MouseScrollEvent},
    },
    roc_integration::Roc,
};
use roc_platform_core::roc_std::{RocList, RocResult, RocStr};

dynamic_lib::define_lib! {
    name = ScriptLib,
    path_env_var = "SCRIPT_LIB_PATH",
    fallback_path = "./libscript";

    unsafe fn roc__setup_scene_extern_1_exposed(ctx_bytes: RocList<u8>) -> RocResult<(), RocStr>;
    unsafe fn roc__update_world_extern_1_exposed(ctx_bytes: RocList<u8>) -> RocResult<(), RocStr>;
    unsafe fn roc__handle_keyboard_event_extern_1_exposed(ctx_bytes: RocList<u8>, event_bytes: RocList<u8>) -> RocResult<(), RocStr>;
    unsafe fn roc__handle_mouse_button_event_extern_1_exposed(ctx_bytes: RocList<u8>, event_bytes: RocList<u8>) -> RocResult<(), RocStr>;
    unsafe fn roc__handle_mouse_drag_event_extern_1_exposed(ctx_bytes: RocList<u8>, event_bytes: RocList<u8>) -> RocResult<(), RocStr>;
    unsafe fn roc__handle_mouse_scroll_event_extern_1_exposed(ctx_bytes: RocList<u8>, event_bytes: RocList<u8>) -> RocResult<(), RocStr>;
}

pub(crate) fn setup_scene(ctx: SetupContext) -> Result<()> {
    // Ensure no lock is being held on the game in case the script calls back
    // into our API
    assert_game_not_accessed();

    let mut ctx_bytes = RocList::from_slice(&[0; SetupContext::SERIALIZED_SIZE]);
    ctx.write_roc_bytes(ctx_bytes.as_mut_slice())?;

    from_roc_result(unsafe { ScriptLib::acquire().roc__setup_scene_extern_1_exposed(ctx_bytes) })
}

pub(crate) fn update_world(ctx: UpdateContext) -> Result<()> {
    assert_game_not_accessed();

    let mut ctx_bytes = RocList::from_slice(&[0; UpdateContext::SERIALIZED_SIZE]);
    ctx.write_roc_bytes(ctx_bytes.as_mut_slice())?;

    from_roc_result(unsafe { ScriptLib::acquire().roc__update_world_extern_1_exposed(ctx_bytes) })
}

pub(crate) fn handle_keyboard_event(ctx: InputContext, event: KeyboardEvent) -> Result<()> {
    assert_game_not_accessed();

    let mut ctx_bytes = RocList::from_slice(&[0; InputContext::SERIALIZED_SIZE]);
    ctx.write_roc_bytes(ctx_bytes.as_mut_slice())?;

    let mut event_bytes = RocList::from_slice(&[0; KeyboardEvent::SERIALIZED_SIZE]);
    event.write_roc_bytes(event_bytes.as_mut_slice())?;

    from_roc_result(unsafe {
        ScriptLib::acquire().roc__handle_keyboard_event_extern_1_exposed(ctx_bytes, event_bytes)
    })
    .with_context(|| format!("Failed handling keyboard event {event:?}"))
}

pub(crate) fn handle_mouse_button_event(ctx: InputContext, event: MouseButtonEvent) -> Result<()> {
    assert_game_not_accessed();

    let mut ctx_bytes = RocList::from_slice(&[0; InputContext::SERIALIZED_SIZE]);
    ctx.write_roc_bytes(ctx_bytes.as_mut_slice())?;

    let mut event_bytes = RocList::from_slice(&[0; MouseButtonEvent::SERIALIZED_SIZE]);
    event.write_roc_bytes(event_bytes.as_mut_slice())?;

    from_roc_result(unsafe {
        ScriptLib::acquire().roc__handle_mouse_button_event_extern_1_exposed(ctx_bytes, event_bytes)
    })
    .with_context(|| format!("Failed handling mouse button event {event:?}"))
}

pub(crate) fn handle_mouse_drag_event(ctx: InputContext, event: MouseDragEvent) -> Result<()> {
    assert_game_not_accessed();

    let mut ctx_bytes = RocList::from_slice(&[0; InputContext::SERIALIZED_SIZE]);
    ctx.write_roc_bytes(ctx_bytes.as_mut_slice())?;

    let mut event_bytes = RocList::from_slice(&[0; MouseDragEvent::SERIALIZED_SIZE]);
    event.write_roc_bytes(event_bytes.as_mut_slice())?;

    from_roc_result(unsafe {
        ScriptLib::acquire().roc__handle_mouse_drag_event_extern_1_exposed(ctx_bytes, event_bytes)
    })
    .with_context(|| format!("Failed handling mouse drag event {event:?}"))
}

pub(crate) fn handle_mouse_scroll_event(ctx: InputContext, event: MouseScrollEvent) -> Result<()> {
    assert_game_not_accessed();

    let mut ctx_bytes = RocList::from_slice(&[0; InputContext::SERIALIZED_SIZE]);
    ctx.write_roc_bytes(ctx_bytes.as_mut_slice())?;

    let mut event_bytes = RocList::from_slice(&[0; MouseScrollEvent::SERIALIZED_SIZE]);
    event.write_roc_bytes(event_bytes.as_mut_slice())?;

    from_roc_result(unsafe {
        ScriptLib::acquire().roc__handle_mouse_scroll_event_extern_1_exposed(ctx_bytes, event_bytes)
    })
    .with_context(|| format!("Failed handling mouse scroll event {event:?}"))
}

fn from_roc_result<T>(res: RocResult<T, RocStr>) -> Result<T> {
    Result::from(res).map_err(|error| anyhow!("{error}"))
}
