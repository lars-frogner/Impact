//! Render commands.

pub mod ambient_light_pass;
pub mod clearing_pass;
pub mod depth_prepass;
pub mod directional_light_pass;
pub mod geometry_pass;
pub mod postprocessing_pass;
pub mod render_attachment_texture_copy_command;
pub mod shadow_map_update_passes;
pub mod skybox_pass;
pub mod storage_buffer_result_copy_command;

use crate::attachment::RenderAttachmentQuantity;
use impact_gpu::{
    shader::Shader,
    timestamp_query::{TimestampQueryRegistry, external::ExternalGPUSpanGuard},
    wgpu,
};
use std::borrow::Cow;

/// The meaning of a specific value in the stencil buffer.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StencilValue {
    Background = 0,
    NonPhysicalModel = 1,
    PhysicalModel = 2,
}

pub const STANDARD_FRONT_FACE: wgpu::FrontFace = wgpu::FrontFace::Ccw;
pub const INVERTED_FRONT_FACE: wgpu::FrontFace = wgpu::FrontFace::Cw;

pub fn create_render_pipeline_layout(
    device: &wgpu::Device,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
    push_constant_ranges: &[wgpu::PushConstantRange],
    label: &str,
) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts,
        push_constant_ranges,
        label: Some(label),
    })
}

pub fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &Shader,
    vertex_buffer_layouts: &[wgpu::VertexBufferLayout<'_>],
    color_target_states: &[Option<wgpu::ColorTargetState>],
    front_face: wgpu::FrontFace,
    cull_mode: Option<wgpu::Face>,
    polygon_mode: wgpu::PolygonMode,
    unclipped_depth: bool,
    depth_stencil_state: Option<wgpu::DepthStencilState>,
    label: &str,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader.vertex_module(),
            entry_point: Some(shader.vertex_entry_point_name().unwrap()),
            buffers: vertex_buffer_layouts,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: shader
            .fragment_entry_point_name()
            .map(|entry_point| wgpu::FragmentState {
                module: shader.fragment_module(),
                entry_point: Some(entry_point),
                targets: color_target_states,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face,
            cull_mode,
            polygon_mode,
            unclipped_depth,
            conservative: false,
        },
        depth_stencil: depth_stencil_state,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
        label: Some(label),
    })
}

pub fn create_line_list_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &Shader,
    vertex_buffer_layouts: &[wgpu::VertexBufferLayout<'_>],
    color_target_states: &[Option<wgpu::ColorTargetState>],
    depth_stencil_state: Option<wgpu::DepthStencilState>,
    unclipped_depth: bool,
    label: &str,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader.vertex_module(),
            entry_point: Some(shader.vertex_entry_point_name().unwrap()),
            buffers: vertex_buffer_layouts,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: shader
            .fragment_entry_point_name()
            .map(|entry_point| wgpu::FragmentState {
                module: shader.fragment_module(),
                entry_point: Some(entry_point),
                targets: color_target_states,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::LineList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::default(),
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::default(),
            unclipped_depth,
            conservative: false,
        },
        depth_stencil: depth_stencil_state,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
        label: Some(label),
    })
}

pub fn depth_stencil_state_for_depth_test_without_write() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: RenderAttachmentQuantity::depth_texture_format(),
        depth_write_enabled: false,
        depth_compare: wgpu::CompareFunction::Less,
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}

pub fn depth_stencil_state_for_depth_stencil_write() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: RenderAttachmentQuantity::depth_texture_format(),
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::Less,
        // Write the reference stencil value to the stencil map
        // whenever the depth test passes
        stencil: wgpu::StencilState {
            front: wgpu::StencilFaceState {
                compare: wgpu::CompareFunction::Always,
                fail_op: wgpu::StencilOperation::Keep,
                depth_fail_op: wgpu::StencilOperation::Keep,
                pass_op: wgpu::StencilOperation::Replace,
            },
            read_mask: 0xFF,
            write_mask: 0xFF,
            ..Default::default()
        },
        bias: wgpu::DepthBiasState::default(),
    }
}

pub fn depth_stencil_state_for_equal_stencil_testing() -> wgpu::DepthStencilState {
    depth_stencil_state_for_stencil_testing(wgpu::CompareFunction::Equal)
}

pub fn depth_stencil_state_for_stencil_testing(
    compare: wgpu::CompareFunction,
) -> wgpu::DepthStencilState {
    // When we are doing stencil testing, we make the depth test always pass and
    // configure the stencil operations to pass only if the given comparison of the
    // stencil value with the reference value passes
    wgpu::DepthStencilState {
        format: RenderAttachmentQuantity::depth_texture_format(),
        depth_write_enabled: false,
        depth_compare: wgpu::CompareFunction::Always,
        stencil: wgpu::StencilState {
            front: wgpu::StencilFaceState {
                compare,
                fail_op: wgpu::StencilOperation::Keep,
                depth_fail_op: wgpu::StencilOperation::Keep,
                pass_op: wgpu::StencilOperation::Keep,
            },
            read_mask: 0xFF,
            write_mask: 0x00,
            ..Default::default()
        },
        bias: wgpu::DepthBiasState::default(),
    }
}

pub fn additive_blend_state() -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent::default(),
    }
}

/// Returns the render pass as well as a span guard for timestamp writes. The
/// span guard should be dropped immediately after the render pass.
#[track_caller]
pub fn begin_single_render_pass<'a>(
    command_encoder: &'a mut wgpu::CommandEncoder,
    timestamp_recorder: &mut TimestampQueryRegistry<'_>,
    color_attachments: &[Option<wgpu::RenderPassColorAttachment<'_>>],
    depth_stencil_attachment: Option<wgpu::RenderPassDepthStencilAttachment<'_>>,
    label: Cow<'static, str>,
) -> (wgpu::RenderPass<'a>, ExternalGPUSpanGuard) {
    let (timestamp_writes, timestamp_span_guard) =
        timestamp_recorder.register_timestamp_writes_for_single_render_pass(label.clone());

    let render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments,
        depth_stencil_attachment,
        timestamp_writes,
        occlusion_query_set: None,
        label: Some(&label),
    });

    (render_pass, timestamp_span_guard)
}
