pub mod tracy;

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

    #[macro_export]
    macro_rules! span {
        ($name: expr) => {};
    }
}

#[cfg(feature = "tracy")]
mod tracy_impl {
    pub use super::tracy::{frame_mark, span};

    use super::tracy;

    #[inline]
    pub fn initialize() {
        tracy::Client::start();
    }

    #[inline]
    pub fn set_thread_name(name: &str) {
        tracy::Client::running()
            .expect("Tracy client not running")
            .set_thread_name(name);
    }
}
