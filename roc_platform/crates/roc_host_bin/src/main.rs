#[allow(clippy::exit)]
fn main() {
    let exit_code = roc_host::rust_main();
    std::process::exit(exit_code);
}
