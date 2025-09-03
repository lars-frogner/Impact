pub mod tracy;

#[cfg(not(feature = "tracy"))]
pub use crate::span;
#[cfg(not(feature = "tracy"))]
pub use no_op_impl::*;

#[cfg(feature = "tracy")]
pub use tracy_impl::*;

#[cfg(not(feature = "tracy"))]
mod no_op_impl {
    #[inline]
    pub fn initialize() {}

    #[inline]
    pub fn set_thread_name(_name: &str) {}

    #[inline]
    pub fn frame_mark() {}

    #[macro_export]
    macro_rules! span {
        ($name: expr) => {{}};
    }
}

#[cfg(feature = "tracy")]
mod tracy_impl {
    pub use tracy_client::{frame_mark, span};

    #[inline]
    pub fn initialize() {
        tracy_client::Client::start();
    }

    #[inline]
    pub fn set_thread_name(name: &str) {
        tracy_client::Client::running()
            .expect("Tracy client not running")
            .set_thread_name(name);
    }
}
