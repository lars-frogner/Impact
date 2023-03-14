//! Management of light source data for rendering.

use crate::{
    geometry::{CollectionChange, UniformBuffer},
    rendering::{
        buffer::{self, UniformBufferable},
        texture::{CascadedShadowMapTexture, ShadowCubemapTexture},
        uniform::{UniformRenderBufferManager, UniformTransferResult},
        CascadeIdx, CoreRenderingSystem, DirectionalLightShaderInput, LightShaderInput,
        PointLightShaderInput, RenderingConfig,
    },
    scene::{
        DirectionalLight, LightID, LightStorage, LightType, PointLight, MAX_SHADOW_MAP_CASCADES,
    },
};
use impact_utils::ConstStringHash64;
use std::mem;

/// Manager of the set of uniform render buffers holding light source render
/// data. Also manages the bind groups for these buffers and for associated
/// shadow map textures.
#[derive(Debug)]
pub struct LightRenderBufferManager {
    point_light_render_buffer_manager: UniformRenderBufferManagerWithLightIDs,
    directional_light_render_buffer_manager: UniformRenderBufferManagerWithLightIDs,
    point_light_shadow_map_texture: ShadowCubemapTexture,
    directional_light_shadow_map_texture: CascadedShadowMapTexture,
    light_bind_group_layout: wgpu::BindGroupLayout,
    light_bind_group: wgpu::BindGroup,
    point_light_shadow_map_bind_group_layout: wgpu::BindGroupLayout,
    point_light_shadow_map_bind_group: wgpu::BindGroup,
    directional_light_shadow_map_bind_group_layout: wgpu::BindGroupLayout,
    directional_light_shadow_map_bind_group: wgpu::BindGroup,
    point_light_shader_input: LightShaderInput,
    directional_light_shader_input: LightShaderInput,
}

#[derive(Debug)]
struct UniformRenderBufferManagerWithLightIDs {
    uniform_render_buffer_manager: UniformRenderBufferManager,
    light_ids: Vec<LightID>,
}

impl LightRenderBufferManager {
    const POINT_LIGHT_BINDING: u32 = 0;
    const POINT_LIGHT_SHADOW_MAP_TEXTURE_BINDING: u32 = 1;
    const POINT_LIGHT_SHADOW_MAP_SAMPLER_BINDING: u32 = 2;

    const DIRECTIONAL_LIGHT_BINDING: u32 = 3;
    const DIRECTIONAL_LIGHT_SHADOW_MAP_TEXTURE_BINDING: u32 = 4;
    const DIRECTIONAL_LIGHT_SHADOW_MAP_SAMPLER_BINDING: u32 = 5;

    const LIGHT_IDX_PUSH_CONSTANT_RANGE_START: u32 = 0;
    const LIGHT_IDX_PUSH_CONSTANT_RANGE_END: u32 =
        Self::LIGHT_IDX_PUSH_CONSTANT_RANGE_START + mem::size_of::<u32>() as u32;

    const CASCADE_IDX_PUSH_CONSTANT_RANGE_START: u32 = Self::LIGHT_IDX_PUSH_CONSTANT_RANGE_END;
    const CASCADE_IDX_PUSH_CONSTANT_RANGE_END: u32 =
        Self::CASCADE_IDX_PUSH_CONSTANT_RANGE_START + mem::size_of::<CascadeIdx>() as u32;

    /// Creates a new manager with render buffers initialized from the given
    /// [`LightStorage`].
    pub fn for_light_storage(
        core_system: &CoreRenderingSystem,
        light_storage: &LightStorage,
        config: &RenderingConfig,
    ) -> Self {
        let point_light_render_buffer_manager =
            UniformRenderBufferManagerWithLightIDs::for_uniform_buffer(
                core_system,
                light_storage.point_light_buffer(),
            );
        let directional_light_render_buffer_manager =
            UniformRenderBufferManagerWithLightIDs::for_uniform_buffer(
                core_system,
                light_storage.directional_light_buffer(),
            );

        let point_light_shadow_map_texture = ShadowCubemapTexture::new(
            core_system,
            config.point_light_shadow_map_resolution,
            "Point light shadow cubemap texture",
        );

        let directional_light_shadow_map_texture = CascadedShadowMapTexture::new(
            core_system,
            config.directional_light_shadow_map_resolution,
            MAX_SHADOW_MAP_CASCADES,
            "Directional light cascaded shadow map texture",
        );

        let light_bind_group_layout = Self::create_light_bind_group_layout(core_system.device());

        let light_bind_group = Self::create_light_bind_group(
            core_system.device(),
            &point_light_render_buffer_manager,
            &directional_light_render_buffer_manager,
            &light_bind_group_layout,
        );

        let point_light_shadow_map_bind_group_layout =
            Self::create_point_light_shadow_map_bind_group_layout(core_system.device());

        let point_light_shadow_map_bind_group = Self::create_point_light_shadow_map_bind_group(
            core_system.device(),
            &point_light_shadow_map_texture,
            &point_light_shadow_map_bind_group_layout,
        );

        let directional_light_shadow_map_bind_group_layout =
            Self::create_directional_light_shadow_map_bind_group_layout(core_system.device());

        let directional_light_shadow_map_bind_group =
            Self::create_directional_light_shadow_map_bind_group(
                core_system.device(),
                &directional_light_shadow_map_texture,
                &directional_light_shadow_map_bind_group_layout,
            );

        let point_light_shader_input =
            Self::create_point_light_shader_input(&point_light_render_buffer_manager);

        let directional_light_shader_input =
            Self::create_directional_light_shader_input(&directional_light_render_buffer_manager);

        Self {
            point_light_render_buffer_manager,
            directional_light_render_buffer_manager,
            point_light_shadow_map_texture,
            directional_light_shadow_map_texture,
            light_bind_group_layout,
            light_bind_group,
            point_light_shadow_map_bind_group_layout,
            point_light_shadow_map_bind_group,
            directional_light_shadow_map_bind_group_layout,
            directional_light_shadow_map_bind_group,
            point_light_shader_input,
            directional_light_shader_input,
        }
    }

    /// Returns the slice of IDs of all the [`PointLight`]s currently residing
    /// in the point light render buffer.
    pub fn point_light_ids(&self) -> &[LightID] {
        self.point_light_render_buffer_manager.light_ids()
    }

    /// Returns the slice of IDs of all the [`DirectionalLight`]scurrently
    /// residing in directional light the render buffer.
    pub fn directional_light_ids(&self) -> &[LightID] {
        self.directional_light_render_buffer_manager.light_ids()
    }

    /// Returns a reference to the shadow cubemap texture for point lights.
    pub fn point_light_shadow_map_texture(&self) -> &ShadowCubemapTexture {
        &self.point_light_shadow_map_texture
    }

    /// Returns a reference to the cascaded shadow map texture for directional
    /// lights.
    pub fn directional_light_shadow_map_texture(&self) -> &CascadedShadowMapTexture {
        &self.directional_light_shadow_map_texture
    }

    /// Returns a reference to the bind group layout for the set of light
    /// uniform buffers.
    pub fn light_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.light_bind_group_layout
    }

    /// Returns a reference to the bind group for the set of light uniform
    /// buffers.
    pub fn light_bind_group(&self) -> &wgpu::BindGroup {
        &self.light_bind_group
    }

    /// Returns the bind group layout for the shadow map texture for the given
    /// light type.
    pub fn shadow_map_bind_group_layout_for_light_type(
        &self,
        light_type: LightType,
    ) -> &wgpu::BindGroupLayout {
        match light_type {
            LightType::PointLight => &self.point_light_shadow_map_bind_group_layout,
            LightType::DirectionalLight => &self.directional_light_shadow_map_bind_group_layout,
        }
    }

    /// Returns the bind group for the shadow map texture for the given light
    /// type.
    pub fn shadow_map_bind_group_for_light_type(&self, light_type: LightType) -> &wgpu::BindGroup {
        match light_type {
            LightType::PointLight => &self.point_light_shadow_map_bind_group,
            LightType::DirectionalLight => &self.directional_light_shadow_map_bind_group,
        }
    }

    /// Returns the input required for accessing light data of the given type in
    /// a shader.
    pub fn shader_input_for_light_type(&self, light_type: LightType) -> &LightShaderInput {
        match light_type {
            LightType::PointLight => &self.point_light_shader_input,
            LightType::DirectionalLight => &self.directional_light_shader_input,
        }
    }

    /// Returns the push constant range that will contain the light index after
    /// [`set_light_idx_push_constant`] is called.
    pub const fn light_idx_push_constant_range() -> wgpu::PushConstantRange {
        wgpu::PushConstantRange {
            stages: wgpu::ShaderStages::VERTEX_FRAGMENT,
            range: Self::LIGHT_IDX_PUSH_CONSTANT_RANGE_START
                ..Self::LIGHT_IDX_PUSH_CONSTANT_RANGE_END,
        }
    }

    /// Returns the push constant range that will contain the the light index
    /// and cascade index after [`set_light_idx_push_constant`] and
    /// [`set_cascade_idx_push_constant`] is called.
    pub const fn light_idx_and_cascade_idx_push_constant_range() -> wgpu::PushConstantRange {
        wgpu::PushConstantRange {
            stages: wgpu::ShaderStages::VERTEX_FRAGMENT,
            range: Self::LIGHT_IDX_PUSH_CONSTANT_RANGE_START
                ..Self::CASCADE_IDX_PUSH_CONSTANT_RANGE_END,
        }
    }

    /// Finds the index of the light with the given ID in the light type's
    /// uniform buffer and writes it to the appropriate push constant range for
    /// the given render pass.
    ///
    /// # Panics
    /// If no light with the given ID is present in the relevant uniform buffer.
    pub fn set_light_idx_push_constant(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        light_type: LightType,
        light_id: LightID,
    ) {
        let light_idx = match light_type {
            LightType::PointLight => &self.point_light_render_buffer_manager,
            LightType::DirectionalLight => &self.directional_light_render_buffer_manager,
        }
        .find_idx_of_light_with_id(light_id)
        .expect("Tried to set light index push constant for missing light");

        let light_idx = u32::try_from(light_idx).unwrap();

        render_pass.set_push_constants(
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            Self::LIGHT_IDX_PUSH_CONSTANT_RANGE_START,
            bytemuck::bytes_of(&light_idx),
        );
    }

    /// Writes the given cascade index to the appropriate push constant range
    /// for the given render pass.
    pub fn set_cascade_idx_push_constant(
        render_pass: &mut wgpu::RenderPass<'_>,
        cascade_idx: CascadeIdx,
    ) {
        render_pass.set_push_constants(
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            Self::CASCADE_IDX_PUSH_CONSTANT_RANGE_START,
            bytemuck::bytes_of(&cascade_idx),
        );
    }

    /// Ensures that the light uniform buffers are in sync with the light data
    /// in the given light storage. Will also recreate the bind group and update
    /// the shader input if any of the render buffers had to be reallocated.
    pub fn sync_with_light_storage(
        &mut self,
        core_system: &CoreRenderingSystem,
        light_storage: &LightStorage,
    ) {
        let point_light_transfer_result = self
            .point_light_render_buffer_manager
            .transfer_uniforms_to_render_buffer(core_system, light_storage.point_light_buffer());

        let directional_light_transfer_result = self
            .directional_light_render_buffer_manager
            .transfer_uniforms_to_render_buffer(
                core_system,
                light_storage.directional_light_buffer(),
            );

        if point_light_transfer_result == UniformTransferResult::CreatedNewBuffer
            || directional_light_transfer_result == UniformTransferResult::CreatedNewBuffer
        {
            // Recreate light bind group and shader input
            self.light_bind_group = Self::create_light_bind_group(
                core_system.device(),
                &self.point_light_render_buffer_manager,
                &self.directional_light_render_buffer_manager,
                &self.light_bind_group_layout,
            );

            if point_light_transfer_result == UniformTransferResult::CreatedNewBuffer {
                self.point_light_shader_input =
                    Self::create_point_light_shader_input(&self.point_light_render_buffer_manager);
            }

            if directional_light_transfer_result == UniformTransferResult::CreatedNewBuffer {
                self.directional_light_shader_input = Self::create_directional_light_shader_input(
                    &self.directional_light_render_buffer_manager,
                );
            }
        }
    }

    fn create_light_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                PointLight::create_bind_group_layout_entry(Self::POINT_LIGHT_BINDING),
                DirectionalLight::create_bind_group_layout_entry(Self::DIRECTIONAL_LIGHT_BINDING),
            ],
            label: Some("Light bind group layout"),
        })
    }

    fn create_light_bind_group(
        device: &wgpu::Device,
        point_light_render_buffer_manager: &UniformRenderBufferManagerWithLightIDs,
        directional_light_render_buffer_manager: &UniformRenderBufferManagerWithLightIDs,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                point_light_render_buffer_manager
                    .manager()
                    .create_bind_group_entry(Self::POINT_LIGHT_BINDING),
                directional_light_render_buffer_manager
                    .manager()
                    .create_bind_group_entry(Self::DIRECTIONAL_LIGHT_BINDING),
            ],
            label: Some("Light bind group"),
        })
    }

    fn create_point_light_shadow_map_bind_group_layout(
        device: &wgpu::Device,
    ) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                ShadowCubemapTexture::create_texture_bind_group_layout_entry(
                    Self::POINT_LIGHT_SHADOW_MAP_TEXTURE_BINDING,
                ),
                ShadowCubemapTexture::create_sampler_bind_group_layout_entry(
                    Self::POINT_LIGHT_SHADOW_MAP_SAMPLER_BINDING,
                ),
            ],
            label: Some("Point light shadow cubemap bind group layout"),
        })
    }

    fn create_point_light_shadow_map_bind_group(
        device: &wgpu::Device,
        point_light_shadow_map_texture: &ShadowCubemapTexture,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                point_light_shadow_map_texture
                    .create_texture_bind_group_entry(Self::POINT_LIGHT_SHADOW_MAP_TEXTURE_BINDING),
                point_light_shadow_map_texture
                    .create_sampler_bind_group_entry(Self::POINT_LIGHT_SHADOW_MAP_SAMPLER_BINDING),
            ],
            label: Some("Point light shadow cubemap bind group"),
        })
    }

    fn create_directional_light_shadow_map_bind_group_layout(
        device: &wgpu::Device,
    ) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                CascadedShadowMapTexture::create_texture_bind_group_layout_entry(
                    Self::DIRECTIONAL_LIGHT_SHADOW_MAP_TEXTURE_BINDING,
                ),
                CascadedShadowMapTexture::create_sampler_bind_group_layout_entry(
                    Self::DIRECTIONAL_LIGHT_SHADOW_MAP_SAMPLER_BINDING,
                ),
            ],
            label: Some("Directional light shadow map bind group layout"),
        })
    }

    fn create_directional_light_shadow_map_bind_group(
        device: &wgpu::Device,
        directional_light_shadow_map_texture: &CascadedShadowMapTexture,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                directional_light_shadow_map_texture.create_texture_bind_group_entry(
                    Self::DIRECTIONAL_LIGHT_SHADOW_MAP_TEXTURE_BINDING,
                ),
                directional_light_shadow_map_texture.create_sampler_bind_group_entry(
                    Self::DIRECTIONAL_LIGHT_SHADOW_MAP_SAMPLER_BINDING,
                ),
            ],
            label: Some("Directional light shadow map bind group"),
        })
    }

    fn create_point_light_shader_input(
        point_light_render_buffer_manager: &UniformRenderBufferManagerWithLightIDs,
    ) -> LightShaderInput {
        LightShaderInput::PointLight(PointLightShaderInput {
            uniform_binding: Self::POINT_LIGHT_BINDING,
            max_light_count: point_light_render_buffer_manager
                .manager()
                .max_uniform_count() as u64,
            shadow_map_texture_and_sampler_binding: (
                Self::POINT_LIGHT_SHADOW_MAP_TEXTURE_BINDING,
                Self::POINT_LIGHT_SHADOW_MAP_SAMPLER_BINDING,
            ),
        })
    }

    fn create_directional_light_shader_input(
        directional_light_render_buffer_manager: &UniformRenderBufferManagerWithLightIDs,
    ) -> LightShaderInput {
        LightShaderInput::DirectionalLight(DirectionalLightShaderInput {
            uniform_binding: Self::DIRECTIONAL_LIGHT_BINDING,
            max_light_count: directional_light_render_buffer_manager
                .manager()
                .max_uniform_count() as u64,
            shadow_map_texture_and_sampler_binding: (
                Self::DIRECTIONAL_LIGHT_SHADOW_MAP_TEXTURE_BINDING,
                Self::DIRECTIONAL_LIGHT_SHADOW_MAP_SAMPLER_BINDING,
            ),
        })
    }
}

impl UniformRenderBufferManagerWithLightIDs {
    /// Creates a new manager with a render buffer and list of light IDs
    /// initialized from the given uniform buffer.
    fn for_uniform_buffer<U>(
        core_system: &CoreRenderingSystem,
        uniform_buffer: &UniformBuffer<LightID, U>,
    ) -> Self
    where
        U: UniformBufferable,
    {
        Self {
            uniform_render_buffer_manager: UniformRenderBufferManager::for_uniform_buffer(
                core_system,
                uniform_buffer,
            ),
            light_ids: uniform_buffer.valid_uniform_ids().to_vec(),
        }
    }

    fn manager(&self) -> &UniformRenderBufferManager {
        &self.uniform_render_buffer_manager
    }

    fn light_ids(&self) -> &[LightID] {
        &self.light_ids
    }

    fn find_idx_of_light_with_id(&self, light_id: LightID) -> Option<usize> {
        self.light_ids.iter().position(|&id| id == light_id)
    }

    fn transfer_uniforms_to_render_buffer<U>(
        &mut self,
        core_system: &CoreRenderingSystem,
        uniform_buffer: &UniformBuffer<LightID, U>,
    ) -> UniformTransferResult
    where
        U: UniformBufferable,
    {
        match uniform_buffer.change() {
            CollectionChange::Count => {
                self.light_ids = uniform_buffer.valid_uniform_ids().to_vec();
            }
            CollectionChange::Contents => {
                self.light_ids
                    .copy_from_slice(uniform_buffer.valid_uniform_ids());
            }
            CollectionChange::None => {}
        }

        self.uniform_render_buffer_manager
            .transfer_uniforms_to_render_buffer(core_system, uniform_buffer)
    }
}

impl UniformBufferable for PointLight {
    const ID: ConstStringHash64 = ConstStringHash64::new("Point light");

    fn create_bind_group_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        buffer::create_uniform_buffer_bind_group_layout_entry(
            binding,
            wgpu::ShaderStages::VERTEX_FRAGMENT,
        )
    }
}

impl UniformBufferable for DirectionalLight {
    const ID: ConstStringHash64 = ConstStringHash64::new("Directional light");

    fn create_bind_group_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        buffer::create_uniform_buffer_bind_group_layout_entry(
            binding,
            wgpu::ShaderStages::VERTEX_FRAGMENT,
        )
    }
}
