//! Utility macros.

/// Implements [`ResourceHandle`](crate::ResourceHandle) for a newtype wrapper
/// around [`SlotKey`](impact_containers::SlotKey).
#[macro_export]
macro_rules! impl_ResourceHandle_for_newtype {
    ($handle:ty) => {
        impl From<::impact_containers::SlotKey> for $handle {
            fn from(key: ::impact_containers::SlotKey) -> Self {
                Self(key)
            }
        }

        impl From<$handle> for ::impact_containers::SlotKey {
            fn from(handle: $handle) -> Self {
                handle.0
            }
        }

        impl ::std::fmt::Display for $handle {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}({})", stringify!($handle), self.0)
            }
        }

        impl $crate::ResourceHandle for $handle {}
    };
}
