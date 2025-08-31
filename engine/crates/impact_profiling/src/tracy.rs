#[cfg(feature = "tracy")]
pub use tracy_client::{
    Client, GpuContext, GpuContextType, GpuSpan, frame_mark, set_thread_name, span_location,
};

#[cfg(not(feature = "tracy"))]
pub use no_tracy::*;

#[cfg(not(feature = "tracy"))]
pub mod no_tracy {
    #![allow(clippy::result_unit_err)]

    #[derive(Debug)]
    pub struct Client;

    #[derive(Debug)]
    pub struct GpuContext;

    #[derive(Debug, Clone, Copy)]
    pub enum GpuContextType {
        Vulkan,
        D3D12,
        Metal,
        OpenGL,
    }

    #[derive(Debug)]
    pub struct GpuSpan;

    impl Client {
        #[inline(always)]
        pub const fn start() -> Self {
            Self
        }

        #[inline(always)]
        pub const fn running() -> Option<Self> {
            None
        }

        #[inline(always)]
        pub const fn new_gpu_context(
            &self,
            _: Option<&str>,
            _: GpuContextType,
            _: i64,
            _: f32,
        ) -> Result<GpuContext, ()> {
            Err(())
        }
    }

    impl GpuContext {
        #[inline(always)]
        pub const fn span(&self, _: (&str, &str, u32)) -> Result<GpuSpan, ()> {
            Ok(GpuSpan)
        }
    }

    impl GpuSpan {
        #[inline(always)]
        pub const fn upload_timestamp_start(&self, _: i64) {}
        #[inline(always)]
        pub const fn upload_timestamp_end(&self, _: i64) {}
    }

    #[inline(always)]
    pub const fn span_location(_: &str) -> (&'static str, &'static str, u32) {
        ("", "", 0)
    }

    #[inline(always)]
    pub const fn frame_mark() {}

    #[macro_export]
    macro_rules! set_thread_name {
        ($name:expr) => {};
    }
}
