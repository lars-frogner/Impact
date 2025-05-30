//! Render passes for applying bloom.

use crate::{
    gpu::{
        GraphicsDevice,
        push_constant::{PushConstantGroup, PushConstantVariant},
        query::TimestampQueryRegistry,
        rendering::{
            render_command::{
                RenderAttachmentTextureCopyCommand, additive_blend_state,
                create_postprocessing_render_pipeline,
                create_postprocessing_render_pipeline_layout,
            },
            resource::SynchronizedRenderResources,
        },
        shader::{
            ShaderID, ShaderManager,
            template::{
                bloom_blending::BloomBlendingShaderTemplate,
                bloom_downsampling::BloomDownsamplingShaderTemplate,
                bloom_upsampling_blur::BloomUpsamplingBlurShaderTemplate,
            },
        },
        texture::attachment::{
            RenderAttachmentDescription, RenderAttachmentInputDescription,
            RenderAttachmentInputDescriptionSet,
            RenderAttachmentQuantity::{self, Luminance, LuminanceAux},
            RenderAttachmentSampler, RenderAttachmentTexture, RenderAttachmentTextureManager,
        },
    },
    mesh::{self, VertexAttributeSet},
};
use anyhow::{Result, anyhow};
use approx::abs_diff_ne;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, num::NonZeroU32};

/// Configuration options for bloom.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BloomConfig {
    /// Whether bloom should be enabled when the scene loads.
    pub enabled: bool,
    /// The number of downsamplings to perform during blurring. More
    /// downsamplings will result in stronger blurring.
    pub n_downsamplings: NonZeroU32,
    /// The radius of the blur filter to apply during upsampling. A larger
    /// radius will result in stronger blurring. This should probably be left at
    /// the default value.
    pub blur_filter_radius: f32,
    /// How strongly the blurred luminance should be weighted when blending with
    /// the original luminance. A value of zero will result in no blending,
    /// effectively disabling bloom. A value of one will replace the original
    /// luminance with the blurred luminance. Use a small value for convincing
    /// bloom.
    pub blurred_luminance_weight: f32,
}

#[derive(Debug)]
pub(super) struct BloomRenderCommands {
    n_downsamplings: usize,
    n_upsamplings: usize,
    push_constants: PushConstantGroup,
    input_descriptions: RenderAttachmentInputDescriptionSet,
    downsampling_pipeline: wgpu::RenderPipeline,
    upsampling_blur_pipeline: wgpu::RenderPipeline,
    blending_pipeline: wgpu::RenderPipeline,
    disabled_command: RenderAttachmentTextureCopyCommand,
    config: BloomConfig,
}

impl BloomConfig {
    fn new_config_requires_recreated_commands(&self, other: &Self) -> bool {
        self.n_downsamplings != other.n_downsamplings
            || abs_diff_ne!(
                self.blur_filter_radius,
                other.blur_filter_radius,
                epsilon = 1e-6
            )
            || abs_diff_ne!(
                self.blurred_luminance_weight,
                other.blurred_luminance_weight,
                epsilon = 1e-6
            )
    }
}

impl Default for BloomConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            n_downsamplings: NonZeroU32::new(4).unwrap(),
            blur_filter_radius: 0.005,
            blurred_luminance_weight: 0.04,
        }
    }
}

const _: () = assert!(LuminanceAux.max_mip_level() > 0);

impl BloomRenderCommands {
    pub(super) fn new(
        config: BloomConfig,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
    ) -> Self {
        let n_downsamplings = u32::min(config.n_downsamplings.get(), LuminanceAux.max_mip_level());
        let n_upsamplings = n_downsamplings - 1; // We don't upsample from mip level 1 to 0

        // We will first downsample progressively down to the highest mip level, then
        // progressively upsample back up the mip chain. On every upsample, we add the
        // blurred upsampled luminances to the existing downsampled luminances at that
        // mip level. This should really be a energy-preserving blend, but we don't
        // bother weighting the blends since the blurred image will be blended with
        // the unblurred luminance texture at the end. We instead normalize the blurred
        // luminance during this final bend.
        let blurred_luminance_normalization = 1.0 / n_downsamplings as f32;

        // This is not necessarily the full window dimensions, but the dimensions of the
        // mip level we are outputting to
        let push_constants =
            PushConstantGroup::for_fragment([PushConstantVariant::InverseWindowDimensions]);
        let push_constant_ranges = push_constants.create_ranges();

        let mut input_descriptions =
            RenderAttachmentInputDescriptionSet::with_capacity(n_downsamplings as usize + 1);

        // The first input for downsampling will be the luminance attachment, which will
        // also be an input for the final blending
        input_descriptions.insert_description(
            RenderAttachmentInputDescription::default_for(RenderAttachmentQuantity::Luminance)
                .with_sampler(RenderAttachmentSampler::Filtering),
        );

        // We will be working within the mip chain of the auxiliary luminance attachment
        for mip_level in 1..(n_downsamplings + 1) {
            input_descriptions.insert_description(
                RenderAttachmentInputDescription::default_for(LuminanceAux)
                    .with_sampler(RenderAttachmentSampler::Filtering)
                    .with_mip_level(mip_level),
            );
        }

        render_attachment_texture_manager
            .create_missing_bind_groups_and_layouts(graphics_device, &input_descriptions);

        // Since all input textures will have the same format, we can use the same
        // layout for all the bind groups (mip level does not affect the layout)
        let texture_format = LuminanceAux.texture_format();
        assert_eq!(Luminance.texture_format(), texture_format);
        let bind_group_layout = render_attachment_texture_manager
            .get_render_attachment_texture_bind_group_layout(&input_descriptions.descriptions()[0])
            .unwrap();

        let blurring_pipeline_layout = create_postprocessing_render_pipeline_layout(
            graphics_device.device(),
            &[bind_group_layout],
            &push_constant_ranges,
            "bloom blurring",
        );

        // For the final blending pass we need two bind groups, one for the
        // original luminance and one for the blurred luminance
        let blending_pipeline_layout = create_postprocessing_render_pipeline_layout(
            graphics_device.device(),
            &[bind_group_layout; 2],
            &push_constant_ranges,
            "bloom blending",
        );

        let replace_color_target_state = wgpu::ColorTargetState {
            format: texture_format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::COLOR,
        };

        let additive_color_target_state = wgpu::ColorTargetState {
            format: texture_format,
            blend: Some(additive_blend_state()),
            write_mask: wgpu::ColorWrites::COLOR,
        };

        let downsampling_shader_template = BloomDownsamplingShaderTemplate::new(LuminanceAux);

        let (_, downsampling_shader) = shader_manager.get_or_create_rendering_shader_from_template(
            graphics_device,
            &downsampling_shader_template,
        );

        let downsampling_pipeline = create_postprocessing_render_pipeline(
            graphics_device,
            &blurring_pipeline_layout,
            downsampling_shader,
            &[Some(replace_color_target_state.clone())],
            None,
            "bloom downsampling",
        );

        // We are deliberately assigning a shader ID that does not depend on the
        // configuration parameters that the shader uses (the default generated
        // ID does). This is so that we can update the parameters and overwrite
        // the existing shader rather than creating a new entry.
        let upsampling_blur_shader_template = BloomUpsamplingBlurShaderTemplate::new(
            Self::upsampling_blur_shader_template_id(),
            LuminanceAux,
            config.blur_filter_radius,
        );

        // If the shader exists, it will be overwritten. This ensures that we
        // will not keep using an invalidated shader after updating the
        // configuration parameters.
        let (_, upsampling_blur_shader) = shader_manager
            .insert_and_get_rendering_shader_from_template(
                graphics_device,
                &upsampling_blur_shader_template,
            );

        let upsampling_blur_pipeline = create_postprocessing_render_pipeline(
            graphics_device,
            &blurring_pipeline_layout,
            upsampling_blur_shader,
            &[Some(additive_color_target_state)],
            None,
            "bloom upsampling and blur",
        );

        let blending_shader_template = BloomBlendingShaderTemplate::new(
            Self::blending_shader_template_id(),
            Luminance,
            LuminanceAux,
            blurred_luminance_normalization,
            config.blurred_luminance_weight,
        );

        let (_, blending_shader) = shader_manager.insert_and_get_rendering_shader_from_template(
            graphics_device,
            &blending_shader_template,
        );

        let blending_pipeline = create_postprocessing_render_pipeline(
            graphics_device,
            &blending_pipeline_layout,
            blending_shader,
            &[Some(replace_color_target_state)],
            None,
            "bloom blending",
        );

        let disabled_command = RenderAttachmentTextureCopyCommand::new(
            RenderAttachmentQuantity::Luminance,
            RenderAttachmentQuantity::LuminanceAux,
        );

        Self {
            n_downsamplings: n_downsamplings as usize,
            n_upsamplings: n_upsamplings as usize,
            push_constants,
            input_descriptions,
            downsampling_pipeline,
            upsampling_blur_pipeline,
            blending_pipeline,
            disabled_command,
            config,
        }
    }

    pub(super) fn enabled_mut(&mut self) -> &mut bool {
        &mut self.config.enabled
    }

    pub(super) fn config(&self) -> &BloomConfig {
        &self.config
    }

    pub(super) fn set_config(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        config: BloomConfig,
    ) {
        if self.config.new_config_requires_recreated_commands(&config) {
            *self = Self::new(
                config,
                graphics_device,
                shader_manager,
                render_attachment_texture_manager,
            );
        } else {
            self.config = config;
        }
    }

    pub(super) fn record(
        &self,
        render_resources: &SynchronizedRenderResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        if !self.config.enabled {
            self.disabled_command
                .record(render_attachment_texture_manager, command_encoder);
            return Ok(());
        }

        let [first_timestamp_writes, last_timestamp_writes] = timestamp_recorder
            .register_timestamp_writes_for_first_and_last_of_render_passes(
                self.n_downsamplings + self.n_upsamplings + 1,
                Cow::Borrowed("Bloom passes"),
            );

        let blurred_luminance_texture =
            render_attachment_texture_manager.render_attachment_texture(LuminanceAux);

        // **** Downsampling ****

        for (input_mip_level, input_description) in self
            .input_descriptions
            .descriptions()
            .iter()
            .take(self.n_downsamplings)
            .enumerate()
        {
            let output_mip_level = input_mip_level as u32 + 1;

            let color_attachment = wgpu::RenderPassColorAttachment {
                view: blurred_luminance_texture
                    .texture_view(output_mip_level)
                    .unwrap(),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            };

            let timestamp_writes = if input_mip_level == 0 {
                first_timestamp_writes.clone()
            } else {
                None
            };

            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(color_attachment)],
                depth_stencil_attachment: None,
                timestamp_writes,
                occlusion_query_set: None,
                label: Some(&format!(
                    "Bloom downsampling pass from mip level {} to {}",
                    input_mip_level, output_mip_level
                )),
            });

            render_pass.set_pipeline(&self.downsampling_pipeline);

            self.set_push_constants(
                &mut render_pass,
                blurred_luminance_texture,
                output_mip_level,
            );

            let bind_group = render_attachment_texture_manager
                .get_render_attachment_texture_bind_group(input_description)
                .unwrap();
            render_pass.set_bind_group(0, bind_group, &[]);

            let mesh_id = mesh::screen_filling_quad_mesh_id();

            let mesh_buffer_manager = render_resources
                .get_mesh_buffer_manager(mesh_id)
                .ok_or_else(|| anyhow!("Missing GPU buffer for mesh {}", mesh_id))?;

            let position_buffer = mesh_buffer_manager
                .request_vertex_gpu_buffers(VertexAttributeSet::POSITION)?
                .next()
                .unwrap();

            render_pass.set_vertex_buffer(0, position_buffer.valid_buffer_slice());

            render_pass.set_index_buffer(
                mesh_buffer_manager.index_gpu_buffer().valid_buffer_slice(),
                mesh_buffer_manager.index_format(),
            );

            render_pass.draw_indexed(
                0..u32::try_from(mesh_buffer_manager.n_indices()).unwrap(),
                0,
                0..1,
            );

            log::trace!(
                "Recorded bloom downsampling pass from mip level {} to {}",
                input_mip_level,
                output_mip_level
            );
        }

        // **** Upsampling ****

        for (input_mip_level, input_description) in self
            .input_descriptions
            .descriptions()
            .iter()
            .enumerate()
            .rev()
            .take(self.n_upsamplings)
        {
            let output_mip_level = input_mip_level as u32 - 1;

            let color_attachment = wgpu::RenderPassColorAttachment {
                view: blurred_luminance_texture
                    .texture_view(output_mip_level)
                    .unwrap(),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            };

            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(color_attachment)],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                label: Some(&format!(
                    "Bloom upsampling and blur pass from mip level {} to {}",
                    input_mip_level, output_mip_level
                )),
            });

            render_pass.set_pipeline(&self.upsampling_blur_pipeline);

            self.set_push_constants(
                &mut render_pass,
                blurred_luminance_texture,
                output_mip_level,
            );

            let bind_group = render_attachment_texture_manager
                .get_render_attachment_texture_bind_group(input_description)
                .unwrap();
            render_pass.set_bind_group(0, bind_group, &[]);

            let mesh_id = mesh::screen_filling_quad_mesh_id();

            let mesh_buffer_manager = render_resources
                .get_mesh_buffer_manager(mesh_id)
                .ok_or_else(|| anyhow!("Missing GPU buffer for mesh {}", mesh_id))?;

            let position_buffer = mesh_buffer_manager
                .request_vertex_gpu_buffers(VertexAttributeSet::POSITION)?
                .next()
                .unwrap();

            render_pass.set_vertex_buffer(0, position_buffer.valid_buffer_slice());

            render_pass.set_index_buffer(
                mesh_buffer_manager.index_gpu_buffer().valid_buffer_slice(),
                mesh_buffer_manager.index_format(),
            );

            render_pass.draw_indexed(
                0..u32::try_from(mesh_buffer_manager.n_indices()).unwrap(),
                0,
                0..1,
            );

            log::trace!(
                "Recorded bloom upsampling and blur pass from mip level {} to {}",
                input_mip_level,
                output_mip_level
            );
        }

        // **** Blending ****

        let color_attachment = wgpu::RenderPassColorAttachment {
            view: blurred_luminance_texture.base_texture_view(),
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        };

        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(color_attachment)],
            depth_stencil_attachment: None,
            timestamp_writes: last_timestamp_writes,
            occlusion_query_set: None,
            label: Some("Bloom blending pass"),
        });

        render_pass.set_pipeline(&self.blending_pipeline);

        self.set_push_constants(&mut render_pass, blurred_luminance_texture, 0);

        let luminance_bind_group = render_attachment_texture_manager
            .get_render_attachment_texture_bind_group(&self.input_descriptions.descriptions()[0])
            .unwrap();
        render_pass.set_bind_group(0, luminance_bind_group, &[]);

        let blurred_luminance_bind_group = render_attachment_texture_manager
            .get_render_attachment_texture_bind_group(&self.input_descriptions.descriptions()[1])
            .unwrap();
        render_pass.set_bind_group(1, blurred_luminance_bind_group, &[]);

        let mesh_id = mesh::screen_filling_quad_mesh_id();

        let mesh_buffer_manager = render_resources
            .get_mesh_buffer_manager(mesh_id)
            .ok_or_else(|| anyhow!("Missing GPU buffer for mesh {}", mesh_id))?;

        let position_buffer = mesh_buffer_manager
            .request_vertex_gpu_buffers(VertexAttributeSet::POSITION)?
            .next()
            .unwrap();

        render_pass.set_vertex_buffer(0, position_buffer.valid_buffer_slice());

        render_pass.set_index_buffer(
            mesh_buffer_manager.index_gpu_buffer().valid_buffer_slice(),
            mesh_buffer_manager.index_format(),
        );

        render_pass.draw_indexed(
            0..u32::try_from(mesh_buffer_manager.n_indices()).unwrap(),
            0,
            0..1,
        );

        log::trace!("Recorded bloom blending pass");

        Ok(())
    }

    fn set_push_constants(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        texture: &RenderAttachmentTexture,
        output_mip_level: u32,
    ) {
        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                PushConstantVariant::InverseWindowDimensions,
                || Self::compute_inverse_output_view_size(texture, output_mip_level),
            );
    }

    fn upsampling_blur_shader_template_id() -> ShaderID {
        ShaderID::from_identifier("BloomUpsamplingBlurShaderTemplate")
    }

    fn blending_shader_template_id() -> ShaderID {
        ShaderID::from_identifier("BloomBlendingShaderTemplate")
    }

    fn compute_inverse_output_view_size(
        texture: &RenderAttachmentTexture,
        output_mip_level: u32,
    ) -> [f32; 2] {
        let output_view_size = texture
            .texture()
            .texture()
            .size()
            .mip_level_size(output_mip_level, wgpu::TextureDimension::D2);

        [
            1.0 / (output_view_size.width as f32),
            1.0 / (output_view_size.height as f32),
        ]
    }
}
