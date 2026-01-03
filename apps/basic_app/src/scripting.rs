//! Calling functions in a Roc script.

use anyhow::{Context, Result, anyhow};
use impact::{
    command::UserCommand,
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
    unsafe fn roc__command_roundtrip_extern_1_exposed(command_bytes: RocList<u8>) -> RocResult<RocList<u8>, RocStr>;
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

pub fn command_roundtrip(command: UserCommand) -> Result<UserCommand> {
    let mut bytes = RocList::from_slice(&[0; UserCommand::SERIALIZED_SIZE]);
    command.write_roc_bytes(bytes.as_mut_slice())?;

    let returned_bytes = from_roc_result(unsafe {
        ScriptLib::acquire().roc__command_roundtrip_extern_1_exposed(bytes)
    })
    .with_context(|| format!("Failed roundtrip for command {command:?}"))?;

    UserCommand::from_roc_bytes(returned_bytes.as_slice())
}

fn from_roc_result<T>(res: RocResult<T, RocStr>) -> Result<T> {
    Result::from(res).map_err(|error| anyhow!("{error}"))
}

#[cfg(feature = "fuzzing")]
pub mod fuzzing {
    use super::*;
    use anyhow::bail;
    use arbitrary::{Arbitrary, Unstructured};
    use impact::impact_math::random::Rng;

    pub fn fuzz_test_command_roundtrip(
        n_iterations: usize,
        seed: u64,
        verbose: bool,
    ) -> Result<()> {
        if verbose {
            println!(
                "Testing {n_iterations} iterations of `UserCommand` roundtrip via Roc (seed {seed})"
            );
        }

        let mut rng = Rng::with_seed(seed);
        let mut byte_buffer = [0; std::mem::size_of::<UserCommand>()];

        for iteration in 0..n_iterations {
            execute_roundtrip_test(&mut rng, &mut byte_buffer).with_context(|| {
                format!("Failed iteration {iteration} of command roundtrip test with seed {seed}")
            })?;
            if verbose {
                println!("Iteration {iteration} completed");
            }
        }

        Ok(())
    }

    fn execute_roundtrip_test(rng: &mut Rng, byte_buffer: &mut [u8]) -> Result<()> {
        let command = generate_command(rng, byte_buffer)?;
        test_command_roundtrip(command)
    }

    fn generate_command(rng: &mut Rng, byte_buffer: &mut [u8]) -> Result<UserCommand> {
        rng.fill_byte_slice(byte_buffer);
        let command = UserCommand::arbitrary(&mut Unstructured::new(byte_buffer))?;
        Ok(command)
    }

    fn test_command_roundtrip(command: UserCommand) -> Result<()> {
        let returned_command = command_roundtrip(command.clone())?;
        if returned_command != command {
            bail!("Roundtrip changed command from `{command:?}` to `{returned_command:?}`");
        }
        Ok(())
    }
}
