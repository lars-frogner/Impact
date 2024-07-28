//! Application of postprocessing.

pub mod ambient_occlusion;
pub mod capturing;
pub mod gaussian_blur;
pub mod temporal_anti_aliasing;

use crate::{
    gpu::{
        push_constant::{PushConstant, PushConstantVariant},
        rendering::render_command::{
            Blending, DepthMapUsage, RenderCommandSpecification, RenderCommandState,
            RenderPassHints, RenderPassSpecification,
        },
        resource_group::GPUResourceGroupManager,
        shader::{template::SpecificShaderTemplate, ShaderManager},
        storage::StorageGPUBufferManager,
        texture::attachment::{
            OutputAttachmentSampling, RenderAttachmentInputDescriptionSet,
            RenderAttachmentOutputDescription, RenderAttachmentOutputDescriptionSet,
            RenderAttachmentQuantity, RenderAttachmentTextureManager,
        },
        GraphicsDevice,
    },
    mesh::{buffer::VertexBufferable, VertexPosition, SCREEN_FILLING_QUAD_MESH_ID},
};
use ambient_occlusion::AmbientOcclusionConfig;
use capturing::{CapturingCamera, CapturingCameraConfig};
use temporal_anti_aliasing::TemporalAntiAliasingConfig;

/// Manager of materials and render commands for postprocessing effects.
#[derive(Clone, Debug)]
pub struct Postprocessor {
    ambient_occlusion_enabled: bool,
    ambient_occlusion_commands: Vec<RenderCommandSpecification>,
    temporal_anti_aliasing_enabled: bool,
    temporal_anti_aliasing_commands: Vec<RenderCommandSpecification>,
    capturing_camera: CapturingCamera,
}

impl Postprocessor {
    /// Creates a new postprocessor along with the associated render commands
    /// according to the given configuration.
    pub fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        gpu_resource_group_manager: &mut GPUResourceGroupManager,
        storage_gpu_buffer_manager: &mut StorageGPUBufferManager,
        ambient_occlusion_config: &AmbientOcclusionConfig,
        temporal_anti_aliasing_config: &TemporalAntiAliasingConfig,
        capturing_camera_settings: &CapturingCameraConfig,
    ) -> Self {
        let ambient_occlusion_commands =
            ambient_occlusion::create_ambient_occlusion_render_commands(
                graphics_device,
                shader_manager,
                gpu_resource_group_manager,
                ambient_occlusion_config,
            );

        let temporal_anti_aliasing_commands =
            temporal_anti_aliasing::create_temporal_anti_aliasing_render_commands(
                graphics_device,
                shader_manager,
                gpu_resource_group_manager,
                temporal_anti_aliasing_config,
            );

        let capturing_camera = CapturingCamera::new(
            graphics_device,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            storage_gpu_buffer_manager,
            capturing_camera_settings,
        );

        Self {
            ambient_occlusion_enabled: ambient_occlusion_config.initially_enabled,
            ambient_occlusion_commands,
            temporal_anti_aliasing_enabled: temporal_anti_aliasing_config.initially_enabled,
            temporal_anti_aliasing_commands,
            capturing_camera,
        }
    }

    /// Returns an iterator over the specifications for all postprocessing
    /// render commands, in the order in which they are to be performed.
    pub fn render_commands(&self) -> impl Iterator<Item = RenderCommandSpecification> + '_ {
        assert_eq!(self.ambient_occlusion_commands.len(), 3);
        assert_eq!(self.temporal_anti_aliasing_commands.len(), 2);
        self.ambient_occlusion_commands
            .iter()
            .cloned()
            .chain(self.capturing_camera.render_commands_before_tone_mapping())
            .chain(self.temporal_anti_aliasing_commands.iter().cloned())
            .chain(self.capturing_camera.render_commands_from_tone_mapping())
    }

    /// Returns an iterator over the current states of all postprocessing render
    /// commands, in the same order as from [`Self::render_commands`].
    pub fn render_command_states(&self) -> impl Iterator<Item = RenderCommandState> + '_ {
        assert_eq!(self.ambient_occlusion_commands.len(), 3);
        assert_eq!(self.temporal_anti_aliasing_commands.len(), 2);
        [
            !self.ambient_occlusion_enabled,
            self.ambient_occlusion_enabled,
            self.ambient_occlusion_enabled,
        ]
        .into_iter()
        .map(RenderCommandState::active_if)
        .chain(
            self.capturing_camera
                .render_command_states_before_tone_mapping(),
        )
        .chain(
            [true, self.temporal_anti_aliasing_enabled]
                .into_iter()
                .map(RenderCommandState::active_if),
        )
        .chain(
            self.capturing_camera
                .render_command_states_from_tone_mapping(),
        )
    }

    /// Returns a reference to the capturing camera.
    pub fn capturing_camera(&self) -> &CapturingCamera {
        &self.capturing_camera
    }

    /// Whether ambient occlusion is enabled.
    pub fn ambient_occlusion_enabled(&self) -> bool {
        self.ambient_occlusion_enabled
    }

    /// Whether temporal anti-aliasing is enabled.
    pub fn temporal_anti_aliasing_enabled(&self) -> bool {
        self.temporal_anti_aliasing_enabled
    }

    /// Returns a mutable reference to the capturing camera.
    pub fn capturing_camera_mut(&mut self) -> &mut CapturingCamera {
        &mut self.capturing_camera
    }

    /// Toggles ambient occlusion.
    pub fn toggle_ambient_occlusion(&mut self) {
        self.ambient_occlusion_enabled = !self.ambient_occlusion_enabled;
    }

    /// Toggles temporal anti-aliasing.
    pub fn toggle_temporal_anti_aliasing(&mut self) {
        self.temporal_anti_aliasing_enabled = !self.temporal_anti_aliasing_enabled;
    }
}

fn create_passthrough_render_pass(
    graphics_device: &GraphicsDevice,
    shader_manager: &mut ShaderManager,
    input_render_attachment_quantity: RenderAttachmentQuantity,
    output_render_attachment_quantity: RenderAttachmentQuantity,
    output_attachment_sampling: OutputAttachmentSampling,
    blending: Blending,
    depth_map_usage: DepthMapUsage,
) -> RenderCommandSpecification {
    let (input_texture_binding, input_sampler_binding) =
        input_render_attachment_quantity.bindings();

    let shader_id = shader_manager
        .get_or_create_rendering_shader_from_template(
            graphics_device,
            SpecificShaderTemplate::Passthrough,
            &[
                (
                    "position_location",
                    VertexPosition::BINDING_LOCATION.to_string(),
                ),
                ("input_texture_binding", input_texture_binding.to_string()),
                ("input_sampler_binding", input_sampler_binding.to_string()),
            ],
        )
        .unwrap();

    let input_render_attachments =
        RenderAttachmentInputDescriptionSet::with_defaults(input_render_attachment_quantity.flag());

    let output_render_attachments = RenderAttachmentOutputDescriptionSet::single(
        output_render_attachment_quantity,
        RenderAttachmentOutputDescription::default()
            .with_sampling(output_attachment_sampling)
            .with_blending(blending),
    );

    RenderCommandSpecification::RenderPass(RenderPassSpecification {
        explicit_mesh_id: Some(*SCREEN_FILLING_QUAD_MESH_ID),
        explicit_shader_id: Some(shader_id),
        input_render_attachments,
        output_render_attachments,
        push_constants: PushConstant::new(
            PushConstantVariant::InverseWindowDimensions,
            wgpu::ShaderStages::FRAGMENT,
        )
        .into(),
        hints: RenderPassHints::NO_DEPTH_PREPASS.union(RenderPassHints::NO_CAMERA),
        label: format!(
            "Passthrough pass: {} -> {}",
            input_render_attachment_quantity, output_render_attachment_quantity
        ),
        depth_map_usage,
        ..Default::default()
    })
}
