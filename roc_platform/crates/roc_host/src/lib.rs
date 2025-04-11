//! Implementation of the host.
//! The host contains code that calls the Roc main function and provides the
//! Roc app with functions to allocate memory and execute effects such as
//! writing to stdio or making HTTP requests.

use core::ffi::c_void;
use roc_io_error::IOErr;
use roc_std::{RocResult, RocStr};
use std::io::Write;

/// # Safety
/// This function is unsafe.
#[no_mangle]
pub unsafe extern "C" fn roc_alloc(size: usize, _alignment: u32) -> *mut c_void {
    libc::malloc(size)
}

/// # Safety
/// This function is unsafe.
#[no_mangle]
pub unsafe extern "C" fn roc_realloc(
    c_ptr: *mut c_void,
    new_size: usize,
    _old_size: usize,
    _alignment: u32,
) -> *mut c_void {
    libc::realloc(c_ptr, new_size)
}

/// # Safety
/// This function is unsafe.
#[no_mangle]
pub unsafe extern "C" fn roc_dealloc(c_ptr: *mut c_void, _alignment: u32) {
    libc::free(c_ptr);
}

/// # Safety
/// This function is unsafe.
#[allow(clippy::exit)]
#[no_mangle]
pub unsafe extern "C" fn roc_panic(msg: *mut RocStr, tag_id: u32) {
    match tag_id {
        0 => {
            eprintln!("Roc standard library hit a panic: {}", &*msg);
        }
        1 => {
            eprintln!("Application hit a panic: {}", &*msg);
        }
        _ => unreachable!(),
    }
    std::process::exit(1);
}

/// # Safety
/// This function is unsafe.
#[no_mangle]
pub unsafe extern "C" fn roc_dbg(loc: *mut RocStr, msg: *mut RocStr, src: *mut RocStr) {
    eprintln!("[{}] {} = {}", &*loc, &*src, &*msg);
}

/// # Safety
/// This function is unsafe.
#[no_mangle]
pub unsafe extern "C" fn roc_memset(dst: *mut c_void, c: i32, n: usize) -> *mut c_void {
    libc::memset(dst, c, n)
}

/// # Safety
/// This function is unsafe.
#[cfg(unix)]
#[no_mangle]
pub unsafe extern "C" fn roc_getppid() -> libc::pid_t {
    libc::getppid()
}

/// # Safety
/// This function is unsafe.
#[cfg(unix)]
#[no_mangle]
pub unsafe extern "C" fn roc_mmap(
    addr: *mut libc::c_void,
    len: libc::size_t,
    prot: libc::c_int,
    flags: libc::c_int,
    fd: libc::c_int,
    offset: libc::off_t,
) -> *mut libc::c_void {
    libc::mmap(addr, len, prot, flags, fd, offset)
}

/// # Safety
/// This function is unsafe.
#[cfg(unix)]
#[no_mangle]
pub unsafe extern "C" fn roc_shm_open(
    name: *const libc::c_char,
    oflag: libc::c_int,
    mode: libc::mode_t,
) -> libc::c_int {
    libc::shm_open(name, oflag, mode as libc::c_uint)
}

// Protect our functions from the vicious GC.
// This is specifically a problem with static compilation and musl.
// TODO: remove all of this when we switch to effect interpreter.
pub fn init() {
    let funcs: &[*const extern "C" fn()] = &[
        roc_alloc as _,
        roc_realloc as _,
        roc_dealloc as _,
        roc_panic as _,
        roc_dbg as _,
        roc_memset as _,
        roc_fx_stdout_line as _,
        roc_fx_impact_run as _,
        roc_fx_f32_to_bits as _,
        roc_fx_f64_to_bits as _,
        roc_fx_f32_from_bits as _,
        roc_fx_f64_from_bits as _,
    ];
    #[allow(forgetting_references)]
    std::mem::forget(std::hint::black_box(funcs));
    if cfg!(unix) {
        let unix_funcs: &[*const extern "C" fn()] =
            &[roc_getppid as _, roc_mmap as _, roc_shm_open as _];
        #[allow(forgetting_references)]
        std::mem::forget(std::hint::black_box(unix_funcs));
    }
}

#[no_mangle]
pub extern "C" fn rust_main() -> i32 {
    extern "C" {
        #[link_name = "roc__main_for_host_1_exposed"]
        pub fn roc_main_for_host(arg_not_used: i32) -> i32;
    }

    init();

    return unsafe { roc_main_for_host(0) };

    let exit_code = match roc_impact::run() {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{}", error);
            1
        }
    };

    exit_code
}

#[no_mangle]
pub extern "C" fn roc_fx_stdout_line(line: &RocStr) -> RocResult<(), IOErr> {
    let stdout = std::io::stdout();

    let mut handle = stdout.lock();

    handle
        .write_all(line.as_bytes())
        .and_then(|()| handle.write_all(b"\n"))
        .and_then(|()| handle.flush())
        .map_err(|io_err| io_err.into())
        .into()
}

#[no_mangle]
pub extern "C" fn roc_fx_impact_run() -> RocResult<(), RocStr> {
    to_roc_result(roc_impact::run())
}

#[no_mangle]
pub extern "C" fn roc_fx_f32_to_bits(value: f32) -> u32 {
    roc_core::f32_to_bits(value)
}

#[no_mangle]
pub extern "C" fn roc_fx_f64_to_bits(value: f64) -> u64 {
    roc_core::f64_to_bits(value)
}

#[no_mangle]
pub extern "C" fn roc_fx_f32_from_bits(bits: u32) -> f32 {
    roc_core::f32_from_bits(bits)
}

#[no_mangle]
pub extern "C" fn roc_fx_f64_from_bits(bits: u64) -> f64 {
    roc_core::f64_from_bits(bits)
}

fn to_roc_result<T>(res: anyhow::Result<T>) -> RocResult<T, RocStr> {
    res.map_err(|err| err.to_string().as_str().into()).into()
}
