/// This function is the entry point for the program, it will be linked by roc
/// using the legacy linker to produce the final executable.
#[no_mangle]
pub extern "C" fn main() -> i32 {
    roc_host::rust_main()
}
