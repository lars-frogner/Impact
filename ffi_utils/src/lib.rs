//! Utilities for making FFI less error prone.

#[macro_export]
macro_rules! define_ffi {
    (
        name = $object:ident,
        lib_path_env = $lib_path_env:expr,
        lib_path_default = $lib_path_default:expr,
        $($symbol:ident => $func:ty,)+
    ) => {
        ::paste::paste! {
            $(
                #[allow(non_camel_case_types)]
                type $symbol = $func;
            )*

            #[allow(non_snake_case)]
            struct $object {
                $(
                    $symbol: ::libloading::Symbol<'static, $symbol>,
                )*
            }

            static [<__LIB_FOR_ $object:upper>]: ::std::sync::LazyLock<::anyhow::Result<::libloading::Library>> = ::std::sync::LazyLock::new(|| {
                use ::anyhow::Context;

                let mut library_path = match ::std::env::var($lib_path_env).map(::std::path::PathBuf::from) {
                    Ok(lib_path) => lib_path,
                    Err(_) => ::std::env::current_exe()?
                        .parent()
                        .unwrap()
                        .join($lib_path_default)
                };
                library_path = library_path
                    .canonicalize()
                    .with_context(|| format!("Failed to resolve library path {}", library_path.display()))?;

                log::debug!("Loading dynamic library at {}", library_path.display());
                Ok(unsafe { ::libloading::Library::new(library_path)? })
            });

            #[allow(non_upper_case_globals)]
            static $object: ::std::sync::LazyLock<::anyhow::Result<$object>> = ::std::sync::LazyLock::new(
                $object::load
            );

            impl $object {
                fn load() -> ::anyhow::Result<Self> {
                    let lib = [<__LIB_FOR_ $object:upper>].as_ref().map_err(|error| ::anyhow::anyhow!("{:#}", error))?;

                    Ok(Self {
                        $(
                            $symbol: {
                                log::debug!("Loading symbol {}", stringify!($symbol));
                                unsafe { lib.get::<$symbol>(stringify!($symbol).as_bytes())? }
                            },
                        )*
                    })
                }

                fn call<T>(call: impl FnOnce(&$object) -> T, map_err: impl FnOnce(&::anyhow::Error) -> T) -> T {
                    match $object.as_ref() {
                        Ok(ffi) => call(ffi),
                        Err(error) => map_err(error),
                    }
                }
            }
        }
    };
}
