# Adding a function `foo` for calling into the application or engine crate from Roc

1. Implement a normal Rust version of `foo` somewhere in the application or engine crate.

2. Implement a wrapper function `foo` in `api.rs` in the application crate. The arguments and return type (within the `Result`) must be FFI-compatible, which means that complex types typically will have to be taken/returned as byte slices. The `foo` wrapper in `api.rs` is responsible for the conversion between Rust types and their corresponding Roc-compatible binary representation.

3. Implement an FFI function (i.e. an `extern "C" fn` annotated with `#[unsafe(no_mangle)]`) `roc_foo` in `api/ffi/roc.rs` in the application crate. This function will be loaded as a symbol and called directly by the Roc platform. The arguments and return types must be FFI-compatible primitive types or Roc-owned composite types (from `roc_std`). The `roc_foo` function should convert the Roc-native arguments to Rust-native types and forward to `foo` in `api.rs` as well as converting the return value back to a Roc-native type.

> **NB:** Be aware that the function must never attempt to transfer ownership of memory between Roc and Rust. The memory of heap-allocated input arguments is managed by the Roc allocator and should thus never be deallocated in Rust. Likewise, values containing pointers to heap memory allocated in Rust should never be returned from the function.

4. Add an entry for the `roc_foo` symbol in the `define_ffi!` invocation in `lib.rs` in the application's `roc_platform`. Include the exact function signature of `roc_foo` from `api/ffi/roc.rs`. This enables the platform crate, which links dynamically with the application crate, to load the symbol for `roc_foo`.

5. Implement the FFI function `roc_fx_foo` in the same `lib.rs` file. This function just acts as a stable entrypoint wrapping the loaded function pointer. It should have the exact same signature as `roc_foo`. Note that the `roc_fx_` part of the name is absolutely necessary for the Roc application to be able to find the symbol when linking with the platform.

6. Add `foo!` to the list of `hosted` Roc functions in `roc_platform/api/Platform.roc`. Then write out the Roc type declaration for the `foo!` function below, matching the signature of `roc_fx_foo`. The platform Roc API can now expose `foo!` directly to Roc applications, or expose a higher-level wrapper.

# Adding a function `bar` for calling into Roc from the application or engine crate

1. Go to the Roc platform header in `roc_platform/api/main.roc` and add the Roc type declaration for `bar!` in the `requires` field. Any Roc application using the platform will then have to provide an implementation of `bar!`.

2. In the same file, implement a function `bar_extern!` wrapping a call to `bar!` with an FFI-compatible interface. Add `bar_extern!` to the `provides` list in the platform header above. This ensures that the shared library consisting of the Roc application and platform will expose a symbol for the `bar_extern!` function. Roc will name this symbol `roc__bar_extern_1_exposed`.

3. Add an entry for the `roc__bar_extern_1_exposed` symbol in the `define_ffi!` invocation in `scripting.rs` in the application crate. Include the exact function signature (now in Rust parlance) of `bar_extern!` from `roc_platform/api/main.roc`.

4. In the same file, implement a function `bar` wrapping a call through the loaded function pointer for `bar_extern!` and exposing a standard Rust interface. The `bar` function should convert arguments to Roc-native types before calling into Roc as well as converting the Roc-native return value back to a Rust-native type. Make sure that the function doesn't attempt to transfer ownership of memory between Rust and Roc.
