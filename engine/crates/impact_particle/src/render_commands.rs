//! Render commands for particles.

use crate::{
    gpu_resource::{
        ParticleGPUResources, ParticleImmediate, ParticleImmediateGroup,
        ParticleMaterialGPUResources, ParticleResourceRegistries, ParticleSimulationGPUResources,
    },
    material::ParticleMaterialID,
    shader_templates::particle_geometry::ParticleGeometryShaderTemplate,
};
use anyhow::{Result, anyhow};
use impact_camera::gpu_resource::CameraGPUResource;
use impact_gpu::{
    bind_group_layout::BindGroupLayoutRegistry,
    device::GraphicsDevice,
    immediate::ImmediateStager,
    shader::{ShaderManager, template::SpecificShaderTemplate},
    wgpu,
};
use impact_rendering::{
    BasicRenderingConfig,
    immediate::BasicImmediate,
    postprocessing::Postprocessor,
    render_command::{self, STANDARD_FRONT_FACE, geometry_pass::GeometryPass},
    resource::BasicGPUResources,
    surface::RenderingSurface,
};

/// GPU commands that should be executed prior to rendering particles.
#[derive(Debug)]
pub struct ParticleRenderCommands {
    geometry_pipeline: ParticleGeometryPipeline,
}

/// Pipeline for rendering the geometric and material properties of particles
/// into the G-buffer attachments. Should be executed as part of the
/// [`GeometryPass`].
#[derive(Debug)]
struct ParticleGeometryPipeline {
    shader_template: ParticleGeometryShaderTemplate,
    immediates: ParticleImmediateGroup,
    color_target_states: Vec<Option<wgpu::ColorTargetState>>,
    depth_stencil_state: Option<wgpu::DepthStencilState>,
    pipeline_layout: wgpu::PipelineLayout,
    pipeline: wgpu::RenderPipeline,
}

impl ParticleRenderCommands {
    /// Initializes the pipelines for particle rendering, including creating the
    /// required shaders.
    pub fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        resource_registries: &impl ParticleResourceRegistries,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
        geometry_pass: &GeometryPass,
        config: &BasicRenderingConfig,
    ) -> Option<Self> {
        let material_registry = resource_registries.particle_material();

        if material_registry.n_particle_materials() == 0 {
            // No materials configured, so no particle rendering
            return None;
        }

        let geometry_pipeline = ParticleGeometryPipeline::new(
            graphics_device,
            shader_manager,
            resource_registries,
            bind_group_layout_registry,
            geometry_pass.color_target_states().to_vec(),
            Some(geometry_pass.depth_stencil_state().clone()),
            config,
        );

        Some(Self { geometry_pipeline })
    }

    /// Recreates pipelines that are out of sync with the current rendering
    /// configuration.
    pub fn sync_with_config(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &ShaderManager,
        config: &BasicRenderingConfig,
    ) {
        self.geometry_pipeline
            .sync_with_config(graphics_device, shader_manager, config);
    }

    /// Makes the particle draw calls for the geometry pass.
    pub fn draw_for_geometry_pass<GR>(
        &self,
        rendering_surface: &RenderingSurface,
        gpu_resources: &GR,
        postprocessor: &Postprocessor,
        frame_counter: u32,
        render_pass: &mut wgpu::RenderPass<'_>,
    ) -> Result<()>
    where
        GR: BasicGPUResources + ParticleGPUResources,
    {
        self.geometry_pipeline.draw(
            rendering_surface,
            gpu_resources,
            postprocessor,
            frame_counter,
            render_pass,
        )
    }
}

impl ParticleGeometryPipeline {
    fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        resource_registries: &impl ParticleResourceRegistries,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
        color_target_states: Vec<Option<wgpu::ColorTargetState>>,
        depth_stencil_state: Option<wgpu::DepthStencilState>,
        config: &BasicRenderingConfig,
    ) -> Self {
        let material_registry = resource_registries.particle_material();
        let shader_template =
            ParticleGeometryShaderTemplate::new(material_registry.n_particle_materials());

        shader_manager
            .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

        let immediates = ParticleGeometryShaderTemplate::immediates();

        let camera_bind_group_layout = CameraGPUResource::get_or_create_bind_group_layout(
            graphics_device,
            bind_group_layout_registry,
        );

        let material_bind_group_layout =
            ParticleMaterialGPUResources::get_or_create_bind_group_layout(
                graphics_device,
                bind_group_layout_registry,
            );

        let particle_buffer_bind_group_layout =
            ParticleSimulationGPUResources::get_or_create_particle_buffer_bind_group_layout(
                graphics_device,
                bind_group_layout_registry,
            );

        let pipeline_layout = render_command::create_render_pipeline_layout(
            graphics_device.device(),
            &[
                Some(&camera_bind_group_layout),
                Some(&material_bind_group_layout),
                Some(&particle_buffer_bind_group_layout),
            ],
            immediates.total_size(),
            "Particle geometry pass render pipeline layout",
        );

        let pipeline = Self::create_pipeline(
            graphics_device,
            shader_manager,
            &shader_template,
            &pipeline_layout,
            &color_target_states,
            depth_stencil_state.clone(),
            config,
        );

        Self {
            shader_template,
            immediates,
            color_target_states,
            depth_stencil_state,
            pipeline_layout,
            pipeline,
        }
    }

    fn create_pipeline(
        graphics_device: &GraphicsDevice,
        shader_manager: &ShaderManager,
        shader_template: &ParticleGeometryShaderTemplate,
        pipeline_layout: &wgpu::PipelineLayout,
        color_target_states: &[Option<wgpu::ColorTargetState>],
        depth_stencil_state: Option<wgpu::DepthStencilState>,
        config: &BasicRenderingConfig,
    ) -> wgpu::RenderPipeline {
        let shader = &shader_manager.rendering_shaders[&shader_template.shader_id()];

        let vertex_buffer_layouts = [Some(
            ParticleMaterialGPUResources::quad_texture_coord_buffer_layout(),
        )];

        render_command::create_render_pipeline(
            graphics_device.device(),
            pipeline_layout,
            shader,
            &vertex_buffer_layouts,
            color_target_states,
            STANDARD_FRONT_FACE,
            Some(wgpu::Face::Back),
            if config.wireframe_mode_on {
                wgpu::PolygonMode::Line
            } else {
                wgpu::PolygonMode::Fill
            },
            false,
            depth_stencil_state,
            "Particle geometry pass render pipeline",
        )
    }

    fn sync_with_config(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &ShaderManager,
        config: &BasicRenderingConfig,
    ) {
        self.pipeline = Self::create_pipeline(
            graphics_device,
            shader_manager,
            &self.shader_template,
            &self.pipeline_layout,
            &self.color_target_states,
            self.depth_stencil_state.clone(),
            config,
        );
    }

    fn stage_constant_immediates(
        stager: &mut ImmediateStager<'_, ParticleImmediate>,
        rendering_surface: &RenderingSurface,
        camera_gpu_resource: &CameraGPUResource,
        postprocessor: &Postprocessor,
        effective_time_step_duration: f32,
        frame_counter: u32,
    ) {
        stager.stage_immediate_data_if_present(
            ParticleImmediate::Rendering(BasicImmediate::WorldToCameraRotation),
            || camera_gpu_resource.world_to_camera_rotation_immediate(),
        );

        stager.stage_immediate_data_if_present(
            ParticleImmediate::Rendering(BasicImmediate::PreviousWorldToCameraRotation),
            || camera_gpu_resource.previous_world_to_camera_rotation_immediate(),
        );

        stager.stage_immediate_data_if_present(
            ParticleImmediate::Rendering(BasicImmediate::WorldToCameraTranslation),
            || camera_gpu_resource.world_to_camera_translation_immediate(),
        );

        stager.stage_immediate_data_if_present(
            ParticleImmediate::Rendering(BasicImmediate::PreviousWorldToCameraTranslation),
            || camera_gpu_resource.previous_world_to_camera_translation_immediate(),
        );

        stager.stage_immediate_data_if_present(
            ParticleImmediate::Rendering(BasicImmediate::InverseWindowDimensions),
            || rendering_surface.inverse_window_dimensions_immediate(),
        );

        stager.stage_immediate_data_if_present(
            ParticleImmediate::Rendering(BasicImmediate::FrameCounter),
            || frame_counter,
        );

        stager.stage_immediate_data_if_present(
            ParticleImmediate::Rendering(BasicImmediate::Exposure),
            || postprocessor.capturing_camera().exposure_immediate(),
        );

        stager.stage_immediate_data_if_present(ParticleImmediate::TimeStepDuration, || {
            effective_time_step_duration
        });
    }

    fn stage_per_emitter_immediates(
        stager: &mut ImmediateStager<'_, ParticleImmediate>,
        material_id: ParticleMaterialID,
    ) {
        stager.stage_immediate_data_if_present(ParticleImmediate::MaterialIdx, || {
            material_id.idx_immediate()
        });
    }

    fn draw<GR>(
        &self,
        rendering_surface: &RenderingSurface,
        gpu_resources: &GR,
        postprocessor: &Postprocessor,
        frame_counter: u32,
        render_pass: &mut wgpu::RenderPass<'_>,
    ) -> Result<()>
    where
        GR: BasicGPUResources + ParticleGPUResources,
    {
        let simulation_resources = gpu_resources.particle_simulations();

        // Skip everything if there are no particle emitters
        if !simulation_resources.has_particle_emitters() {
            return Ok(());
        }

        let camera_gpu_resource = gpu_resources
            .camera()
            .ok_or_else(|| anyhow!("Missing camera GPU resource"))?;

        let material_gpu_resources = gpu_resources
            .particle_materials()
            .ok_or_else(|| anyhow!("Missing particle material GPU resources"))?;

        let particle_buffer = simulation_resources
            .particle_buffer()
            .ok_or_else(|| anyhow!("Missing particle GPU buffer"))?;

        // We don't assign the camera projection uniform bind group here, as it will
        // already have been assigned by the caller

        render_pass.set_pipeline(&self.pipeline);

        render_pass.set_bind_group(1, material_gpu_resources.bind_group(), &[]);

        render_pass.set_bind_group(2, particle_buffer.render_bind_group(), &[]);

        render_pass.set_index_buffer(
            material_gpu_resources
                .quad_index_buffer()
                .valid_buffer_slice(),
            ParticleMaterialGPUResources::quad_index_format(),
        );
        render_pass.set_vertex_buffer(
            0,
            material_gpu_resources
                .quad_texture_coord_buffer()
                .valid_buffer_slice(),
        );

        let mut immediate_stager = self.immediates.create_data_stager();
        Self::stage_constant_immediates(
            &mut immediate_stager,
            rendering_surface,
            camera_gpu_resource,
            postprocessor,
            simulation_resources.effective_time_step_duration(),
            frame_counter,
        );

        for metadata in simulation_resources.all_emitter_metadata() {
            Self::stage_per_emitter_immediates(&mut immediate_stager, metadata.material_id);
            immediate_stager.set_staged_immediate_data(render_pass);

            render_pass.draw_indexed(0..6, 0, metadata.particle_range_u32());
        }

        log::trace!(
            "Made geometry draw calls for {} particle emitters",
            simulation_resources.n_particle_emitters()
        );

        Ok(())
    }
}
