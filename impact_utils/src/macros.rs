/// Utility macros.

/// Creates a [`StringHash`](crate::hash::StringHash) for
/// the given string.
#[macro_export]
macro_rules! hash {
    ($string:literal) => {
        $crate::StringHash::new_with_hash($string, $crate::compute_hash_str_64($string))
    };
    ($string:expr) => {
        $crate::StringHash::new($string)
    };
}

/// Defines a new type with the given name that is a wrapper
/// around a [`StringHash`](crate::hash::StringHash).
#[macro_export]
macro_rules! stringhash_newtype {
    (
        $(#[$attributes:meta])*
        $([$pub:ident])? $name:ident
    ) => {
        $(#[$attributes])*
        #[repr(C)]
        #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, bytemuck::Zeroable, bytemuck::Pod)]
        $($pub)? struct $name($($pub)? $crate::StringHash);

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}
