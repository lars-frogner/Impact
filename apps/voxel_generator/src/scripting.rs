//! Calling functions in a Roc script.

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

    unsafe fn roc__setup_scene_extern_1_exposed(_unused: i32) -> RocResult<(), RocStr>;
    unsafe fn roc__handle_keyboard_event_extern_1_exposed(event_bytes: RocList<u8>) -> RocResult<(), RocStr>;
    unsafe fn roc__handle_mouse_button_event_extern_1_exposed(event_bytes: RocList<u8>) -> RocResult<(), RocStr>;
    unsafe fn roc__handle_mouse_drag_event_extern_1_exposed(event_bytes: RocList<u8>) -> RocResult<(), RocStr>;
    unsafe fn roc__handle_mouse_scroll_event_extern_1_exposed(event_bytes: RocList<u8>) -> RocResult<(), RocStr>;
}

pub fn setup_scene() -> Result<()> {
    from_roc_result(unsafe { ScriptLib::acquire().roc__setup_scene_extern_1_exposed(0) })
        .with_context(|| "Failed scene setup")
}

pub fn handle_keyboard_event(event: KeyboardEvent) -> Result<()> {
    let mut bytes = RocList::from_slice(&[0; KeyboardEvent::SERIALIZED_SIZE]);
    event.write_roc_bytes(bytes.as_mut_slice())?;

    from_roc_result(unsafe {
        ScriptLib::acquire().roc__handle_keyboard_event_extern_1_exposed(bytes)
    })
    .with_context(|| format!("Failed handling keyboard event {event:?}"))
}

pub fn handle_mouse_button_event(event: MouseButtonEvent) -> Result<()> {
    let mut bytes = RocList::from_slice(&[0; MouseButtonEvent::SERIALIZED_SIZE]);
    event.write_roc_bytes(bytes.as_mut_slice())?;

    from_roc_result(unsafe {
        ScriptLib::acquire().roc__handle_mouse_button_event_extern_1_exposed(bytes)
    })
    .with_context(|| format!("Failed handling mouse button event {event:?}"))
}

pub fn handle_mouse_drag_event(event: MouseDragEvent) -> Result<()> {
    let mut bytes = RocList::from_slice(&[0; MouseDragEvent::SERIALIZED_SIZE]);
    event.write_roc_bytes(bytes.as_mut_slice())?;

    from_roc_result(unsafe {
        ScriptLib::acquire().roc__handle_mouse_drag_event_extern_1_exposed(bytes)
    })
    .with_context(|| format!("Failed handling mouse drag event {event:?}"))
}

pub fn handle_mouse_scroll_event(event: MouseScrollEvent) -> Result<()> {
    let mut bytes = RocList::from_slice(&[0; MouseScrollEvent::SERIALIZED_SIZE]);
    event.write_roc_bytes(bytes.as_mut_slice())?;

    from_roc_result(unsafe {
        ScriptLib::acquire().roc__handle_mouse_scroll_event_extern_1_exposed(bytes)
    })
    .with_context(|| format!("Failed handling mouse scroll event {event:?}"))
}

fn from_roc_result<T>(res: RocResult<T, RocStr>) -> Result<T> {
    Result::from(res).map_err(|error| anyhow!("{error}"))
}
