//! Automatic generation of [Roc](roc-lang.org) code for interfacing with the
//! engine.

pub mod meta;

#[cfg(feature = "enabled")]
mod primitives;

#[cfg(feature = "enabled")]
pub mod generate;

/// Derive macro generating an implementation of the [`Roc`](crate::meta::Roc)
/// trait. The macro will infer and register a
/// [`RocTypeDescriptor`](crate::meta::RocTypeDescriptor) for the target type,
/// which is used to [`generate`](crate::generate) the associated Roc code.
///
/// Note that this registration is only performed when the crate hosting the
/// target type has an active feature named `roc_codegen` and the `enabled`
/// feature is active for the [`roc_codegen`] crate.
///
/// Deriving the `Roc` trait will only work for types whose constituent types
/// all implement `Roc` or [`RocPod`](crate::meta::RocPod).
pub use roc_codegen_macros::Roc;

/// Derive macro generating an implementation of the
/// [`RocPod`](crate::meta::RocPod) trait. The macro will infer and register a
/// [`RocTypeDescriptor`](crate::meta::RocTypeDescriptor) for the target type,
/// which is used to [`generate`](crate::generate) the associated Roc code.
///
/// Note that this registration is only performed when the crate hosting the
/// target type has an active feature named `roc_codegen` and the `enabled`
/// feature is active for the [`roc_codegen`] crate.
///
/// The `RocPod` trait represents [`Roc`](crate::meta::Roc) types consisting
/// of Plain Old Data (POD). Derive this trait rather that the `Roc` trait
/// for types implementing [`Pod`](bytemuck::Pod), as this will unlock more
/// use cases for the generated Roc code. In particular, ECS components, which
/// are always POD, should always derive this trait rather than plain `Roc`.
///
/// Deriving the `RocPod` trait will only work for types whose constituent
/// types are all POD and implement `Roc` or `RocPod`.
pub use roc_codegen_macros::RocPod;

pub use meta::RocTypeID;
