//! External consumers of GPU timestamp queries.

use crate::device::GraphicsDevice;
use anyhow::{Result, bail};
use parking_lot::Mutex;
use std::{
    fmt,
    sync::{Arc, Weak},
};

/// An external instrumenting GPU profiler that can be hooked into
/// [`TimestampQueryManager`](super::TimestampQueryManager).
#[derive(Debug)]
pub enum ExternalGPUProfiler {
    None,
    Tracy(TracyGPUProfiler),
}

/// A guard that ensures that a timestamp write span is properly closed when it
/// drops. It should be dropped immediately after the `RenderPass` or
/// `ComputePass` for which the timestamps are written.
#[derive(Debug)]
pub enum ExternalGPUSpanGuard {
    None,
    Tracy(TracyGPUSpanGuard),
}

/// Helper for uploading GPU timestamps to a Tracy server.
pub struct TracyGPUProfiler {
    ctx: impact_profiling::instrumentation::tracy::GpuContext,
    spans: Arc<Mutex<Vec<TracyGPUSpan>>>,
}

struct TracyGPUSpan {
    span: impact_profiling::instrumentation::tracy::GpuSpan,
    start_idx: u32,
    end_idx: u32,
}

#[derive(Debug, Clone)]
pub struct TracyGPUSpanGuard {
    spans: Weak<Mutex<Vec<TracyGPUSpan>>>,
    span_idx: usize,
}

impl ExternalGPUProfiler {
    #[track_caller]
    pub(super) fn add_span(
        &mut self,
        tag: &str,
        start_idx: u32,
        end_idx: u32,
    ) -> ExternalGPUSpanGuard {
        match self {
            Self::None => ExternalGPUSpanGuard::None,
            Self::Tracy(profiler) => profiler.add_span(tag, start_idx, end_idx),
        }
    }

    pub(super) fn load_spans(&mut self, timestamps: &[u64]) {
        match self {
            Self::None => {}
            Self::Tracy(profiler) => profiler.load_spans(timestamps),
        }
    }
}

impl TracyGPUProfiler {
    pub fn new(graphics_device: &GraphicsDevice, name: Option<&str>) -> Result<Self> {
        use impact_profiling::instrumentation::tracy;

        if !graphics_device.supports_features(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS) {
            bail!(
                "Tracy support requires the TIMESTAMP_QUERY_INSIDE_ENCODERS \
                 wgpu feature, which is not available on this graphics device"
            );
        }

        let period_ns_per_tick = graphics_device.queue().get_timestamp_period();

        let ty = match graphics_device.adapter().get_info().backend {
            wgpu::Backend::Vulkan => tracy::GpuContextType::Vulkan,
            wgpu::Backend::Dx12 => tracy::GpuContextType::Direct3D12,
            wgpu::Backend::Gl => tracy::GpuContextType::OpenGL,
            _ => tracy::GpuContextType::Invalid,
        };

        let client = tracy::Client::start();

        let gpu_timestamp = super::obtain_current_gpu_timestamp(graphics_device)? as i64;

        let ctx = client.new_gpu_context(name, ty, gpu_timestamp, period_ns_per_tick)?;

        Ok(Self {
            ctx,
            spans: Arc::new(Mutex::new(Vec::new())),
        })
    }

    #[track_caller]
    fn add_span(&mut self, tag: &str, start_idx: u32, end_idx: u32) -> ExternalGPUSpanGuard {
        let location = std::panic::Location::caller();

        if let Ok(span) = self
            .ctx
            .span_alloc(tag, "", location.file(), location.line())
        {
            let mut spans = self.spans.lock();
            let span_idx = spans.len();
            spans.push(TracyGPUSpan {
                span,
                start_idx,
                end_idx,
            });
            drop(spans);

            ExternalGPUSpanGuard::Tracy(TracyGPUSpanGuard {
                spans: Arc::downgrade(&self.spans),
                span_idx,
            })
        } else {
            ExternalGPUSpanGuard::None
        }
    }

    fn load_spans(&mut self, timestamps: &[u64]) {
        for span in self.spans.lock().drain(..) {
            let start_timestamp = timestamps[span.start_idx as usize];
            let end_timestamp = timestamps[span.end_idx as usize];
            span.span.upload_timestamp_start(start_timestamp as i64);
            span.span.upload_timestamp_end(end_timestamp as i64);
        }
    }
}

impl fmt::Debug for TracyGPUProfiler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TracyGPUProfiler").finish()
    }
}

impl fmt::Debug for TracyGPUSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TracyGPUSpan")
            .field("start_query", &self.start_idx)
            .field("end_query", &self.end_idx)
            .finish()
    }
}

impl Drop for TracyGPUSpanGuard {
    fn drop(&mut self) {
        if let Some(spans) = self.spans.upgrade()
            && let Some(span) = spans.lock().get_mut(self.span_idx)
        {
            span.span.end_zone();
        }
    }
}
