/// Macro for defining an interface to a dynamic library with specific callable
/// symbols. The names and function signatures of the symbols are specified as
/// function declarations in the macro input. The function names must correspond
/// exactly to the symbol names.
///
/// The macro will define a new type using the given `name` and provide
/// associated functions to `load` and `unload` the library. When loading the
/// library, the environment variable specified by `path_env_var` will be
/// checked for the library path. If not set, the path specified by
/// `fallback_path`, which may be absolute or relative to the directory of the
/// executable, is checked instead. The loaded library is stored in a static
/// variable.
///
/// Use the `acquire` function on the generated type to acquire a lock on the
/// loaded library in the static variable. The returned guard will have methods
/// for calling the available symbols.
#[macro_export]
macro_rules! define_lib {
    (
        name = $lib_obj:ident,
        path_env_var = $path_env_var:expr,
        fallback_path = $fallback_path:expr;
        $(
            $( #[$meta:meta] )*
            unsafe fn $symbol:ident($($argn:ident:$argt:ty),*) -> $ret:ty;
        )+
    ) => {
        mod __lib_scope {
            #![allow(non_snake_case, non_upper_case_globals)]

            use super::*;

            /// A dynamic library that can be loaded and unloaded.
            #[derive(Debug)]
            pub struct $lib_obj {
                // SAFETY: We pretend that the symbols have a static lifetime so
                // that we can store them together with the library, but their
                // actual lifetime is bounded by the lifetime of the library.
                // For safety, we need to ensure that the symbols are dropped
                // before the library. Since drop order follows field
                // declaration order, we must declare the symbols first and the
                // library last. We must also ensure that the symbols are
                // private so that they can't be copied out and used after the
                // library is unloaded (which would be allowed by the artificial
                // static lifetime).
                $(
                    $symbol: ::dynamic_lib::Symbol<'static, unsafe extern "C" fn($($argt),*) -> $ret>,
                )*
                _library: ::dynamic_lib::Library,
            }

            // Create a static variable containing a lock on an unloaded (if
            // `None`) or loaded libray. We use the same name as the library
            // type to avoid requiring access to a `paste` macro.
            static $lib_obj: ::dynamic_lib::RwLock<Option<$lib_obj>> = ::dynamic_lib::RwLock::new(None);

            impl ::dynamic_lib::LoadableLibrary for $lib_obj {
                fn new_loaded() -> ::dynamic_lib::Result<Self> {
                    let library = ::dynamic_lib::__from_macro_load_library($path_env_var, $fallback_path)?;
                    Ok(Self {
                        $(
                            $symbol: {
                                // Load the symbol. It will have the same
                                // lifetime as `library`.
                                let symbol = ::dynamic_lib::__from_macro_load_symbol::<
                                    unsafe extern "C" fn($($argt),*) -> $ret
                                >(&library, stringify!($symbol))?;

                                // Transmute the lifetime to static so that we
                                // can store it in the same object as the
                                // library. SAFETY: The declaration order
                                // guarantees that the symbol is dropped before
                                // the library. Defining the object in a private
                                // module with private fields and re-exporting
                                // it ensures that no-one has direct access to
                                // the symbols outside of this macro body, which
                                // would enable them to store the function
                                // pointers and call them after they become
                                // stale.
                                unsafe { ::std::mem::transmute(symbol) }
                            },
                        )*
                        _library: library,
                    })
                }
            }

            impl ::dynamic_lib::DynamicLibrary for $lib_obj {
                /// Loads the dynamic library and associated symbols and stores
                /// them in a static variable. The library can then be accessed
                /// by calling [`Self::acquire`].
                ///
                /// # Errors
                /// Returns an error if:
                ///  - The library is already loaded.
                ///  - The library path can not be resolved.
                ///  - The library can not be loaded.
                fn load() -> ::dynamic_lib::Result<()> {
                    ::dynamic_lib::__from_macro_load(&$lib_obj)
                }

                /// Unloads the dynamic library. Attempts to [`Self::acquire`]
                /// it will result in a panic until the library is loaded again
                /// with [`Self::load`].
                ///
                /// # Errors
                /// Returns an error if the library is not loaded.
                fn unload() -> ::dynamic_lib::Result<()> {
                    ::dynamic_lib::__from_macro_unload(&$lib_obj)
                }
            }

            impl $lib_obj {
                /// Acquires a read lock on the static variable containing the
                /// dynamic library and returns a guard that can be used to
                /// access the library's symbols.
                ///
                /// # Panics
                /// If the library is not currently loaded. Use [`Self::load`]
                /// to load the library.
                #[inline]
                pub fn acquire() -> ::dynamic_lib::MappedRwLockReadGuard<'static, $lib_obj> {
                    ::dynamic_lib::__from_macro_acquire(&$lib_obj)
                }

                /// Acquires a read lock on the static variable containing the
                /// dynamic library and returns a guard that can be used to
                /// access the library's symbols. If the library is not loaded,
                /// this method will load it first.
                ///
                /// # Errors
                /// Returns an error if:
                ///  - The library path can not be resolved.
                ///  - The library can not be loaded.
                #[inline]
                pub fn load_and_acquire() -> ::dynamic_lib::Result<::dynamic_lib::MappedRwLockReadGuard<'static, $lib_obj>> {
                    ::dynamic_lib::__from_macro_load_and_acquire(&$lib_obj)
                }

                // Define methods corresponding to they symbol function
                // signatures so that they can be called after acquiring the
                // loaded library. For safety, we do not expose any function
                // pointers.
                $(
                    $(#[$meta])*
                    #[inline]
                    pub unsafe fn $symbol(&self, $($argn : $argt),*) -> $ret {
                        (self.$symbol)($($argn),*)
                    }
                )*
            }
        }

        pub use self::__lib_scope::$lib_obj;
    };
}
