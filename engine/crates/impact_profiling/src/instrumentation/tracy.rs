#[cfg(feature = "tracy")]
pub use tracy_client::{Client, GpuContext, GpuContextType, GpuSpan, frame_mark, span};

#[cfg(not(feature = "tracy"))]
pub use no_tracy::*;

#[cfg(not(feature = "tracy"))]
pub mod no_tracy {
    #![allow(clippy::result_unit_err)]

    use anyhow::{Result, bail};

    #[derive(Debug)]
    pub struct Client;

    #[derive(Debug)]
    pub struct GpuContext;

    #[derive(Debug, Clone, Copy)]
    pub enum GpuContextType {
        Vulkan,
        Direct3D12,
        OpenGL,
        Invalid,
    }

    #[derive(Debug)]
    pub struct GpuSpan;

    impl Client {
        #[inline(always)]
        pub fn start() -> Self {
            Self
        }

        #[inline(always)]
        pub fn running() -> Option<Self> {
            Some(Self)
        }

        #[inline(always)]
        pub fn set_thread_name(&self, _name: &str) {}

        #[inline(always)]
        pub fn new_gpu_context(
            &self,
            _name: Option<&str>,
            _ty: GpuContextType,
            _gpu_timestamp: i64,
            _period: f32,
        ) -> Result<GpuContext> {
            bail!("`tracy` feature is not enabled")
        }
    }

    impl GpuContext {
        #[inline(always)]
        pub fn span_alloc(
            &self,
            _name: &str,
            _function: &str,
            _file: &str,
            _line: u32,
        ) -> Result<GpuSpan> {
            Ok(GpuSpan)
        }
    }

    impl GpuSpan {
        #[inline(always)]
        pub fn end_zone(&mut self) {}

        #[inline(always)]
        pub fn upload_timestamp_start(&self, _: i64) {}

        #[inline(always)]
        pub fn upload_timestamp_end(&self, _: i64) {}
    }

    #[inline(always)]
    pub fn frame_mark() {}
}
