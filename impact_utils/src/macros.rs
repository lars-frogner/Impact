/// Utility macros.

/// Creates a [`StringHash32`](crate::StringHash32) for
/// the given string.
#[macro_export]
macro_rules! hash32 {
    ($string:literal) => {
        $crate::StringHash32::new_with_hash($string, $crate::compute_hash_str_32($string))
    };
    ($string:expr) => {
        $crate::StringHash32::new($string)
    };
}

/// Creates a [`StringHash64`](crate::StringHash64) for
/// the given string.
#[macro_export]
macro_rules! hash64 {
    ($string:literal) => {
        $crate::StringHash64::new_with_hash($string, $crate::compute_hash_str_64($string))
    };
    ($string:expr) => {
        $crate::StringHash64::new($string)
    };
}

/// Defines a new type with the given name that is a wrapper
/// around a [`StringHash32`](crate::StringHash32).
#[macro_export]
macro_rules! stringhash32_newtype {
    (
        $(#[$attributes:meta])*
        $([$pub:ident])? $name:ident
    ) => {
        $(#[$attributes])*
        #[repr(C)]
        #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, bytemuck::Zeroable, bytemuck::Pod)]
        $($pub)? struct $name($($pub)? $crate::StringHash32);

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

/// Defines a new type with the given name that is a wrapper
/// around a [`StringHash64`](crate::StringHash64).
#[macro_export]
macro_rules! stringhash64_newtype {
    (
        $(#[$attributes:meta])*
        $([$pub:ident])? $name:ident
    ) => {
        $(#[$attributes])*
        #[repr(C)]
        #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, bytemuck::Zeroable, bytemuck::Pod)]
        $($pub)? struct $name($($pub)? $crate::StringHash64);

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

/// Defines a new type with the given name that is a wrapper
/// around a [`Hash64`](crate::Hash64).
#[macro_export]
macro_rules! hash64_newtype {
    (
        $(#[$attributes:meta])*
        $([$pub:ident])? $name:ident
    ) => {
        $(#[$attributes])*
        #[repr(C)]
        #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, bytemuck::Zeroable, bytemuck::Pod)]
        $($pub)? struct $name($($pub)? $crate::Hash64);
    };
}
