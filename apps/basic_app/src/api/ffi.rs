//! FFI functions.

pub mod roc;

use crate::api;
use anyhow::{Result, anyhow};
use std::{slice, str};

/// # Safety
/// The caller must ensure that:
/// - The function does not take ownership of the memory; it will not
///   deallocate or modify it.
/// - See [`slice::from_raw_parts`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn run_with_config_at_path(
    config_path_ptr: *const u8,
    config_path_len: usize,
) -> i32 {
    let config_path = match unsafe { convert_string(config_path_ptr, config_path_len) } {
        Ok(config_path) => config_path,
        Err(error) => {
            eprintln!("Invalid arguments to run_with_config_at_path: {error:#}");
            return 1;
        }
    };
    match api::run_with_config_at_path(config_path) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error:#}");
            1
        }
    }
}

#[cfg(feature = "fuzzing")]
#[unsafe(no_mangle)]
pub extern "C" fn fuzz_test_command_roundtrip(n_iterations: usize, seed: u64, verbose: u8) -> i32 {
    match crate::scripting::fuzzing::fuzz_test_command_roundtrip(n_iterations, seed, verbose != 0) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error:#}");
            1
        }
    }
}

unsafe fn create_slice<'a, T>(slice_ptr: *const T, slice_len: usize) -> &'a [T] {
    if slice_ptr.is_null() {
        &[]
    } else {
        unsafe { slice::from_raw_parts(slice_ptr, slice_len) }
    }
}

unsafe fn convert_string(string_ptr: *const u8, string_len: usize) -> Result<String> {
    let string_bytes = unsafe { create_slice(string_ptr, string_len) };

    str::from_utf8(string_bytes)
        .map(String::from)
        .map_err(|error| anyhow!("Invalid UTF-8: {error}"))
}
