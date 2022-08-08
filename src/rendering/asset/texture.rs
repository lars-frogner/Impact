//! Textures.

mod image;

pub use self::image::ImageTexture;

stringhash_newtype!(
    /// Identifier for specific textures.
    /// Wraps a [`StringHash`](crate::hash::StringHash).
    [pub] TextureID
);
