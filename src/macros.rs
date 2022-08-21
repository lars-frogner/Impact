//! Crate-local macros and utility macros.

macro_rules! with_debug_logging {
    ($message:expr $(,$arg:expr)*; $expression:expr) => {{
        log::debug!(concat!("Begin: ", $message)$(,$arg)*);
        let _result = $expression;
        log::debug!(concat!("Done: ", $message)$(,$arg)*);
        _result
    }};
}

/// Creates a [`StringHash`](crate::hash::StringHash) for
/// the given [`String`] or string literal.
macro_rules! hash {
    ($string:literal) => {
        $crate::hash::StringHash::of_literal($string)
    };
    ($string:expr) => {
        $crate::hash::StringHash::of_owned($string)
    };
}

/// Defines a new type with the given name that is a wrapper
/// around a [`StringHash`](crate::hash::StringHash).
macro_rules! stringhash_newtype {
    (
        $(#[$attributes:meta])*
        $([$pub:ident])? $name:ident
    ) => {
        $(#[$attributes])*
        #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $($pub)? struct $name($($pub)? $crate::hash::StringHash);

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

/// This macro expands to a compile time constant equal
/// to the number of arguments passed to the macro.
#[doc(hidden)]
#[macro_export]
macro_rules! count_ident_args {
    ($($arg:ident),*) => {
        // Ugly hack utilizing that `[]::len` is a `const fn`
        // (the extra "" and -1 are needed for the hack to work for zero args)
        ["", $(stringify!($arg)),*].len() - 1
    };
}
