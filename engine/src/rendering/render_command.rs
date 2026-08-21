//! Render commands.

use crate::{gpu::resource::GPUResourceManager, resource::ResourceManager};
use anyhow::Result;
use impact_gizmo::render_commands::GizmoPasses;
use impact_gpu::{
    bind_group_layout::BindGroupLayoutRegistry, device::GraphicsDevice,
    resource_group::GPUResourceGroupManager, shader::ShaderManager,
    storage::StorageGPUBufferManager, timestamp_query::TimestampQueryRegistry, wgpu,
};
use impact_particle::render_commands::ParticleRenderCommands;
use impact_rendering::{
    BasicRenderingConfig,
    attachment::{RenderAttachmentQuantitySet, RenderAttachmentTextureManager},
    postprocessing::Postprocessor,
    render_command::{
        StencilValue,
        ambient_light_pass::AmbientLightPass,
        clearing_pass::AttachmentClearingPass,
        depth_prepass::DepthPrepass,
        directional_light_pass::DirectionalLightPass,
        geometry_pass::GeometryPass,
        shadow_map_update_passes::{
            OmnidirectionalLightShadowMapUpdatePasses, UnidirectionalLightShadowMapUpdatePasses,
        },
        skybox_pass::SkyboxPass,
    },
    surface::RenderingSurface,
};
use impact_voxel::render_commands::VoxelRenderCommands;

/// Manager of GPU commands for rendering the scene. Postprocessing commands are
/// managed by the [`Postprocessor`], but evoked by this manager.
#[derive(Debug)]
pub struct RenderCommandManager {
    attachment_clearing_pass: AttachmentClearingPass,
    non_physical_model_depth_prepass: DepthPrepass,
    geometry_pass: GeometryPass,
    omnidirectional_light_shadow_map_update_passes: OmnidirectionalLightShadowMapUpdatePasses,
    unidirectional_light_shadow_map_update_passes: UnidirectionalLightShadowMapUpdatePasses,
    ambient_light_pass: AmbientLightPass,
    directional_light_pass: DirectionalLightPass,
    skybox_pass: SkyboxPass,
    particle_render_commands: Option<ParticleRenderCommands>,
    voxel_render_commands: Option<VoxelRenderCommands>,
    gizmo_passes: GizmoPasses,
}

impl RenderCommandManager {
    /// Creates a new render command manager, initializing all
    /// non-postprocessing render commands.
    pub fn new(
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        resource_manager: &ResourceManager,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
        config: &BasicRenderingConfig,
    ) -> Self {
        let attachment_clearing_pass = AttachmentClearingPass::new(
            (RenderAttachmentQuantitySet::DEPTH_STENCIL
                | RenderAttachmentQuantitySet::all().with_clear_color_only())
                - RenderAttachmentQuantitySet::g_buffer(),
        );

        let non_physical_model_depth_prepass = DepthPrepass::new(
            graphics_device,
            shader_manager,
            bind_group_layout_registry,
            StencilValue::NonPhysicalModel,
            config,
        );

        let geometry_pass = GeometryPass::new(config);

        let omnidirectional_light_shadow_map_update_passes =
            OmnidirectionalLightShadowMapUpdatePasses::new(
                graphics_device,
                shader_manager,
                bind_group_layout_registry,
            );

        let unidirectional_light_shadow_map_update_passes =
            UnidirectionalLightShadowMapUpdatePasses::new(
                graphics_device,
                shader_manager,
                bind_group_layout_registry,
            );

        let ambient_light_pass = AmbientLightPass::new(
            graphics_device,
            shader_manager,
            render_attachment_texture_manager,
            resource_manager,
            bind_group_layout_registry,
        );

        let directional_light_pass = DirectionalLightPass::new(
            graphics_device,
            shader_manager,
            render_attachment_texture_manager,
            bind_group_layout_registry,
        );

        let skybox_pass = SkyboxPass::new(graphics_device, shader_manager);

        let particle_render_commands = ParticleRenderCommands::new(
            graphics_device,
            shader_manager,
            resource_manager,
            bind_group_layout_registry,
            &geometry_pass,
            config,
        );

        let voxel_render_commands = VoxelRenderCommands::new(
            graphics_device,
            shader_manager,
            resource_manager,
            bind_group_layout_registry,
            &geometry_pass,
            config,
        );

        let gizmo_passes = GizmoPasses::new(
            graphics_device,
            rendering_surface,
            shader_manager,
            bind_group_layout_registry,
        );

        Self {
            attachment_clearing_pass,
            non_physical_model_depth_prepass,
            geometry_pass,
            omnidirectional_light_shadow_map_update_passes,
            unidirectional_light_shadow_map_update_passes,
            ambient_light_pass,
            directional_light_pass,
            skybox_pass,
            particle_render_commands,
            voxel_render_commands,
            gizmo_passes,
        }
    }

    pub fn sync_with_config(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &ShaderManager,
        config: &BasicRenderingConfig,
    ) {
        self.non_physical_model_depth_prepass.sync_with_config(
            graphics_device,
            shader_manager,
            config,
        );

        self.geometry_pass
            .sync_with_config(graphics_device, shader_manager, config);

        if let Some(particle_render_commands) = &mut self.particle_render_commands {
            particle_render_commands.sync_with_config(graphics_device, shader_manager, config);
        }

        if let Some(voxel_render_commands) = &mut self.voxel_render_commands {
            voxel_render_commands.sync_with_config(graphics_device, shader_manager, config);
        }
    }

    /// Makes sure all the render commands are up to date with the given render
    /// resources.
    ///
    /// # Errors
    /// Returns an error if any of the required GPU resources are missing.
    pub fn sync_with_gpu_resources(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        gpu_resource_manager: &GPUResourceManager,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> Result<()> {
        self.non_physical_model_depth_prepass
            .sync_with_gpu_resources_for_non_physical_models(gpu_resource_manager);

        self.geometry_pass.sync_with_gpu_resources(
            graphics_device,
            shader_manager,
            gpu_resource_manager,
            bind_group_layout_registry,
        )?;

        self.omnidirectional_light_shadow_map_update_passes
            .sync_with_gpu_resources(graphics_device, shader_manager, gpu_resource_manager);

        self.unidirectional_light_shadow_map_update_passes
            .sync_with_gpu_resources(graphics_device, shader_manager, gpu_resource_manager);

        self.ambient_light_pass.sync_with_gpu_resources(
            graphics_device,
            shader_manager,
            gpu_resource_manager,
        );

        self.directional_light_pass.sync_with_gpu_resources(
            graphics_device,
            shader_manager,
            gpu_resource_manager,
        );

        self.skybox_pass.sync_with_gpu_resources(
            graphics_device,
            shader_manager,
            bind_group_layout_registry,
            gpu_resource_manager,
        );

        Ok(())
    }

    /// Records all render commands (including postprocessing commands) that do
    /// not write directly into the surface texture into the given command
    /// encoder.
    ///
    /// # Errors
    /// Returns an error if any of the required GPU resources are missing.
    pub fn record_before_surface(
        &self,
        rendering_surface: &RenderingSurface,
        gpu_resource_manager: &GPUResourceManager,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        storage_gpu_buffer_manager: &StorageGPUBufferManager,
        postprocessor: &Postprocessor,
        frame_counter: u32,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        self.attachment_clearing_pass.record(
            render_attachment_texture_manager,
            timestamp_recorder,
            command_encoder,
        )?;

        if let Some(voxel_render_commands) = &self.voxel_render_commands {
            voxel_render_commands.record_before_geometry_pass(
                gpu_resource_manager,
                timestamp_recorder,
                command_encoder,
            )?;
        }

        self.non_physical_model_depth_prepass.record(
            rendering_surface,
            gpu_resource_manager,
            render_attachment_texture_manager,
            frame_counter,
            timestamp_recorder,
            command_encoder,
        )?;

        let (mut geometry_pass, timestamp_span_guard) = self.geometry_pass.record(
            rendering_surface,
            gpu_resource_manager,
            render_attachment_texture_manager,
            postprocessor,
            frame_counter,
            timestamp_recorder,
            command_encoder,
        )?;

        if let Some(ref mut pass) = geometry_pass {
            if let Some(particle_render_commands) = &self.particle_render_commands {
                particle_render_commands.draw_for_geometry_pass(
                    rendering_surface,
                    gpu_resource_manager,
                    postprocessor,
                    frame_counter,
                    pass,
                )?;
            }
            if let Some(voxel_render_commands) = &self.voxel_render_commands {
                voxel_render_commands.draw_for_geometry_pass(
                    rendering_surface,
                    gpu_resource_manager,
                    postprocessor,
                    frame_counter,
                    pass,
                )?;
            }
        }
        drop(geometry_pass);
        drop(timestamp_span_guard);

        self.omnidirectional_light_shadow_map_update_passes.record(
            gpu_resource_manager,
            timestamp_recorder,
            command_encoder,
            &mut |positive_z_cubemap_face_frustum,
                  instance_range_id,
                  timestamp_recorder,
                  command_encoder| {
                if let Some(voxel_render_commands) = &self.voxel_render_commands {
                    voxel_render_commands
                        .record_before_omnidirectional_light_shadow_cubemap_face_update(
                            positive_z_cubemap_face_frustum,
                            instance_range_id,
                            gpu_resource_manager,
                            timestamp_recorder,
                            command_encoder,
                        )
                } else {
                    Ok(())
                }
            },
            &mut |instance_range_id, render_pass| {
                if self.voxel_render_commands.is_some() {
                    VoxelRenderCommands::record_shadow_map_update(
                        instance_range_id,
                        gpu_resource_manager,
                        render_pass,
                    )
                } else {
                    Ok(())
                }
            },
        )?;

        self.unidirectional_light_shadow_map_update_passes.record(
            gpu_resource_manager,
            timestamp_recorder,
            command_encoder,
            &mut |cascade_frustum, instance_range_id, timestamp_recorder, command_encoder| {
                if let Some(voxel_render_commands) = &self.voxel_render_commands {
                    voxel_render_commands
                        .record_before_unidirectional_light_shadow_map_cascade_update(
                            cascade_frustum,
                            instance_range_id,
                            gpu_resource_manager,
                            timestamp_recorder,
                            command_encoder,
                        )
                } else {
                    Ok(())
                }
            },
            &mut |instance_range_id, render_pass| {
                if self.voxel_render_commands.is_some() {
                    VoxelRenderCommands::record_shadow_map_update(
                        instance_range_id,
                        gpu_resource_manager,
                        render_pass,
                    )
                } else {
                    Ok(())
                }
            },
        )?;

        self.ambient_light_pass.record(
            rendering_surface,
            gpu_resource_manager,
            render_attachment_texture_manager,
            postprocessor,
            timestamp_recorder,
            command_encoder,
        )?;

        self.directional_light_pass.record(
            rendering_surface,
            gpu_resource_manager,
            render_attachment_texture_manager,
            postprocessor,
            timestamp_recorder,
            command_encoder,
        )?;

        self.skybox_pass.record(
            gpu_resource_manager,
            render_attachment_texture_manager,
            postprocessor,
            timestamp_recorder,
            command_encoder,
        )?;

        postprocessor.record_commands_before_surface(
            rendering_surface,
            gpu_resource_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            storage_gpu_buffer_manager,
            frame_counter,
            timestamp_recorder,
            command_encoder,
        )?;

        Ok(())
    }

    /// Records the final render commands that write directly into the
    /// surface texture into the given command encoder.
    ///
    /// # Errors
    /// Returns an error if any of the required GPU resources are missing.
    pub fn record_with_surface(
        &self,
        rendering_surface: &RenderingSurface,
        surface_texture_view: &wgpu::TextureView,
        gpu_resource_manager: &GPUResourceManager,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        postprocessor: &Postprocessor,
        frame_counter: u32,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        postprocessor.record_commands_with_surface(
            rendering_surface,
            surface_texture_view,
            gpu_resource_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            frame_counter,
            timestamp_recorder,
            command_encoder,
        )?;

        self.gizmo_passes.record(
            surface_texture_view,
            gpu_resource_manager,
            render_attachment_texture_manager,
            timestamp_recorder,
            command_encoder,
        )?;

        Ok(())
    }
}
