//! Calling functions in a Roc script.

use anyhow::{Context, Result, anyhow};
use ffi_helpers::define_ffi;
use impact::{
    input::{
        key::KeyboardEvent,
        mouse::{MouseButtonEvent, MouseDragEvent, MouseScrollEvent},
    },
    roc_integration::Roc,
};
use roc_platform_core::roc_std::{RocList, RocResult, RocStr};

define_ffi! {
    name = ScriptFFI,
    lib_path_env = "SCRIPT_LIB_PATH",
    lib_path_default = "./libscript",
    roc__setup_scene_extern_1_exposed => unsafe extern "C" fn(i32) -> RocResult<(), RocStr>,
    roc__handle_keyboard_event_extern_1_exposed => unsafe extern "C" fn(RocList<u8>) -> RocResult<(), RocStr>,
    roc__handle_mouse_button_event_extern_1_exposed => unsafe extern "C" fn(RocList<u8>) -> RocResult<(), RocStr>,
    roc__handle_mouse_drag_event_extern_1_exposed => unsafe extern "C" fn(RocList<u8>) -> RocResult<(), RocStr>,
    roc__handle_mouse_scroll_event_extern_1_exposed => unsafe extern "C" fn(RocList<u8>) -> RocResult<(), RocStr>,
}

pub fn setup_scene() -> Result<()> {
    ScriptFFI::call(
        |ffi| from_roc_result(unsafe { (ffi.roc__setup_scene_extern_1_exposed)(0) }),
        |error| Err(anyhow!("{:#}", error)),
    )
    .with_context(|| "Failed scene setup")
}

pub fn handle_keyboard_event(event: KeyboardEvent) -> Result<()> {
    let mut bytes = RocList::from_slice(&[0; KeyboardEvent::SERIALIZED_SIZE]);
    event.write_roc_bytes(bytes.as_mut_slice())?;

    ScriptFFI::call(
        |ffi| from_roc_result(unsafe { (ffi.roc__handle_keyboard_event_extern_1_exposed)(bytes) }),
        |error| Err(anyhow!("{:#}", error)),
    )
    .with_context(|| format!("Failed handling keyboard event {event:?}"))
}

pub fn handle_mouse_button_event(event: MouseButtonEvent) -> Result<()> {
    let mut bytes = RocList::from_slice(&[0; MouseButtonEvent::SERIALIZED_SIZE]);
    event.write_roc_bytes(bytes.as_mut_slice())?;

    ScriptFFI::call(
        |ffi| {
            from_roc_result(unsafe { (ffi.roc__handle_mouse_button_event_extern_1_exposed)(bytes) })
        },
        |error| Err(anyhow!("{:#}", error)),
    )
    .with_context(|| format!("Failed handling mouse button event {event:?}"))
}

pub fn handle_mouse_drag_event(event: MouseDragEvent) -> Result<()> {
    let mut bytes = RocList::from_slice(&[0; MouseDragEvent::SERIALIZED_SIZE]);
    event.write_roc_bytes(bytes.as_mut_slice())?;

    ScriptFFI::call(
        |ffi| {
            from_roc_result(unsafe { (ffi.roc__handle_mouse_drag_event_extern_1_exposed)(bytes) })
        },
        |error| Err(anyhow!("{:#}", error)),
    )
    .with_context(|| format!("Failed handling mouse drag event {event:?}"))
}

pub fn handle_mouse_scroll_event(event: MouseScrollEvent) -> Result<()> {
    let mut bytes = RocList::from_slice(&[0; MouseScrollEvent::SERIALIZED_SIZE]);
    event.write_roc_bytes(bytes.as_mut_slice())?;

    ScriptFFI::call(
        |ffi| {
            from_roc_result(unsafe { (ffi.roc__handle_mouse_scroll_event_extern_1_exposed)(bytes) })
        },
        |error| Err(anyhow!("{:#}", error)),
    )
    .with_context(|| format!("Failed handling mouse scroll event {event:?}"))
}

fn from_roc_result<T>(res: RocResult<T, RocStr>) -> Result<T> {
    Result::from(res).map_err(|error| anyhow!("{error}"))
}
