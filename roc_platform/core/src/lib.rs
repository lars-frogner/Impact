//! Implementation of the host.
//! The host contains code that calls the Roc main function and provides the
//! Roc app with functions to allocate memory and execute effects such as
//! writing to stdio.

pub use roc_std;

use core::ffi::c_void;
use roc_io_error::IOErr;
use roc_std::{RocResult, RocStr};
use std::io::Write;

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

/// # Safety
/// This function delegates to [`libc::malloc`], and so is equally unsafe.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roc_alloc(size: usize, _alignment: u32) -> *mut c_void {
    unsafe { libc::malloc(size) }
}

/// # Safety
/// This function delegates to [`libc::realloc`], and so is equally unsafe.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roc_realloc(
    c_ptr: *mut c_void,
    new_size: usize,
    _old_size: usize,
    _alignment: u32,
) -> *mut c_void {
    unsafe { libc::realloc(c_ptr, new_size) }
}

/// # Safety
/// This function delegates to [`libc::free`], and so is equally unsafe.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roc_dealloc(c_ptr: *mut c_void, _alignment: u32) {
    unsafe {
        libc::free(c_ptr);
    }
}

/// # Safety
/// This function delegates to [`libc::memset`], and so is equally unsafe.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roc_memset(dst: *mut c_void, c: i32, n: usize) -> *mut c_void {
    unsafe { libc::memset(dst, c, n) }
}

/// # Safety
/// This function delegates to [`libc::getppid`], and so is equally unsafe.
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roc_getppid() -> libc::pid_t {
    unsafe { libc::getppid() }
}

/// # Safety
/// This function delegates to [`libc::mmap`], and so is equally unsafe.
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roc_mmap(
    addr: *mut libc::c_void,
    len: libc::size_t,
    prot: libc::c_int,
    flags: libc::c_int,
    fd: libc::c_int,
    offset: libc::off_t,
) -> *mut libc::c_void {
    unsafe { libc::mmap(addr, len, prot, flags, fd, offset) }
}

/// # Safety
/// This function delegates to [`libc::shm_open`], and so is equally unsafe.
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roc_shm_open(
    name: *const libc::c_char,
    oflag: libc::c_int,
    mode: libc::mode_t,
) -> libc::c_int {
    unsafe { libc::shm_open(name, oflag, mode as libc::c_uint) }
}

/// # Safety
/// ??
#[allow(clippy::exit)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roc_panic(msg: *mut RocStr, tag_id: u32) {
    match tag_id {
        0 => {
            eprintln!("Roc standard library hit a panic: {}", unsafe { &*msg });
        }
        1 => {
            eprintln!("Application hit a panic: {}", unsafe { &*msg });
        }
        _ => unreachable!(),
    }
    std::process::exit(1);
}

/// # Safety
/// ??
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roc_dbg(loc: *mut RocStr, msg: *mut RocStr, src: *mut RocStr) {
    unsafe { eprintln!("[{}] {} = {}", &*loc, &*src, &*msg) };
}

#[unsafe(no_mangle)]
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
