//! Calling functions in a Roc script.

use anyhow::{Context, Result, anyhow};
use ffi_utils::define_ffi;
use impact::{
    command::UserCommand,
    roc_integration::Roc,
    window::input::{key::KeyboardEvent, mouse::MouseButtonEvent},
};
use roc_platform_core::roc_std::{RocList, RocResult, RocStr};

define_ffi! {
    name = ScriptFFI,
    lib_path_env = "SCRIPT_LIB_PATH",
    lib_path_default = "./libscript",
    roc__setup_scene_extern_1_exposed => unsafe extern "C" fn(i32) -> RocResult<(), RocStr>,
    roc__handle_keyboard_event_extern_1_exposed => unsafe extern "C" fn(RocList<u8>) -> RocResult<(), RocStr>,
    roc__handle_mouse_button_event_extern_1_exposed => unsafe extern "C" fn(RocList<u8>) -> RocResult<(), RocStr>,
    roc__command_roundtrip_extern_1_exposed => unsafe extern "C" fn(RocList<u8>) -> RocResult<RocList<u8>, RocStr>,
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

pub fn command_roundtrip(command: UserCommand) -> Result<UserCommand> {
    let mut bytes = RocList::from_slice(&[0; UserCommand::SERIALIZED_SIZE]);
    command.write_roc_bytes(bytes.as_mut_slice())?;

    let returned_bytes = ScriptFFI::call(
        |ffi| from_roc_result(unsafe { (ffi.roc__command_roundtrip_extern_1_exposed)(bytes) }),
        |error| Err(anyhow!("{:#}", error)),
    )
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
    use rand::{Rng, SeedableRng, rngs::StdRng};

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

        let mut rng = StdRng::seed_from_u64(seed);
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

    fn execute_roundtrip_test(rng: &mut StdRng, byte_buffer: &mut [u8]) -> Result<()> {
        let command = generate_command(rng, byte_buffer)?;
        test_command_roundtrip(command)
    }

    fn generate_command(rng: &mut StdRng, byte_buffer: &mut [u8]) -> Result<UserCommand> {
        rng.fill(byte_buffer);
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
