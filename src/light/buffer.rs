//! Buffering of light source data for rendering.

use crate::{
    assert_uniform_valid,
    gpu::{
        rendering::RenderingConfig,
        texture::shadow_map::{CascadeIdx, CascadedShadowMapTexture, ShadowCubemapTexture},
        uniform::{
            self, MultiUniformGPUBuffer, UniformBuffer, UniformBufferable, UniformTransferResult,
        },
        GraphicsDevice,
    },
    light::{
        AmbientLight, LightID, LightStorage, LightType, OmnidirectionalLight, UnidirectionalLight,
        MAX_SHADOW_MAP_CASCADES,
    },
    util::tracking::CollectionChange,
};
use impact_utils::ConstStringHash64;
use std::{mem, sync::OnceLock};

/// Manager of the set of uniform GPU buffers holding light source render
/// data. Also manages the bind groups for these buffers and the associated
/// shadow map textures.
#[derive(Debug)]
pub struct LightGPUBufferManager {
    ambient_light_gpu_buffer: UniformGPUBufferWithLightIDs,
    omnidirectional_light_gpu_buffer: UniformGPUBufferWithLightIDs,
    unidirectional_light_gpu_buffer: UniformGPUBufferWithLightIDs,
    ambient_light_bind_group: wgpu::BindGroup,
    omnidirectional_light_bind_group: wgpu::BindGroup,
    unidirectional_light_bind_group: wgpu::BindGroup,
    omnidirectional_light_shadow_map_manager: OmnidirectionalLightShadowMapManager,
    unidirectional_light_shadow_map_manager: UnidirectionalLightShadowMapManager,
}

/// Manager of the [`ShadowCubemapTexture`]s used by all omnidirectional lights.
#[derive(Debug)]
pub struct OmnidirectionalLightShadowMapManager {
    resolution: u32,
    textures: Vec<ShadowCubemapTexture>,
    light_count: usize,
}

/// Manager of the [`CascadedShadowMapTexture`]s used by all unidirectional
/// lights.
#[derive(Debug)]
pub struct UnidirectionalLightShadowMapManager {
    resolution: u32,
    textures: Vec<CascadedShadowMapTexture>,
    light_count: usize,
}

#[derive(Debug)]
struct UniformGPUBufferWithLightIDs {
    uniform_gpu_buffer: MultiUniformGPUBuffer,
    light_ids: Vec<LightID>,
}

static AMBIENT_LIGHT_BIND_GROUP_LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
static OMNIDIRECTIONAL_LIGHT_BIND_GROUP_LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
static UNIDIRECTIONAL_LIGHT_BIND_GROUP_LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();

static OMNIDIRECTIONAL_LIGHT_SHADOW_MAP_BIND_GROUP_LAYOUT: OnceLock<wgpu::BindGroupLayout> =
    OnceLock::new();
static UNIDIRECTIONAL_LIGHT_SHADOW_MAP_BIND_GROUP_LAYOUT: OnceLock<wgpu::BindGroupLayout> =
    OnceLock::new();

impl LightGPUBufferManager {
    const AMBIENT_LIGHT_VISIBILITY: wgpu::ShaderStages = wgpu::ShaderStages::FRAGMENT;
    const OMNIDIRECTIONAL_LIGHT_VISIBILITY: wgpu::ShaderStages =
        wgpu::ShaderStages::VERTEX_FRAGMENT;
    const UNIDIRECTIONAL_LIGHT_VISIBILITY: wgpu::ShaderStages = wgpu::ShaderStages::VERTEX_FRAGMENT;

    /// The binding location of one of the light uniform buffers.
    pub const fn light_binding() -> u32 {
        0
    }

    /// Creates a new manager with GPU buffers initialized from the given
    /// [`LightStorage`].
    pub fn for_light_storage(
        graphics_device: &GraphicsDevice,
        light_storage: &LightStorage,
        config: &RenderingConfig,
    ) -> Self {
        let ambient_light_gpu_buffer = UniformGPUBufferWithLightIDs::for_uniform_buffer(
            graphics_device,
            light_storage.ambient_light_buffer(),
            Self::AMBIENT_LIGHT_VISIBILITY,
        );
        let omnidirectional_light_gpu_buffer = UniformGPUBufferWithLightIDs::for_uniform_buffer(
            graphics_device,
            light_storage.omnidirectional_light_buffer(),
            Self::OMNIDIRECTIONAL_LIGHT_VISIBILITY,
        );
        let unidirectional_light_gpu_buffer = UniformGPUBufferWithLightIDs::for_uniform_buffer(
            graphics_device,
            light_storage.unidirectional_light_buffer(),
            Self::UNIDIRECTIONAL_LIGHT_VISIBILITY,
        );

        let ambient_light_bind_group_layout =
            Self::get_or_create_ambient_light_bind_group_layout(graphics_device);
        let omnidirectional_light_bind_group_layout =
            Self::get_or_create_omnidirectional_light_bind_group_layout(graphics_device);
        let unidirectional_light_bind_group_layout =
            Self::get_or_create_unidirectional_light_bind_group_layout(graphics_device);

        let ambient_light_bind_group = Self::create_light_bind_group(
            graphics_device.device(),
            &ambient_light_gpu_buffer,
            ambient_light_bind_group_layout,
            "Ambient light bind group",
        );
        let omnidirectional_light_bind_group = Self::create_light_bind_group(
            graphics_device.device(),
            &omnidirectional_light_gpu_buffer,
            omnidirectional_light_bind_group_layout,
            "Omnidirectional light bind group",
        );
        let unidirectional_light_bind_group = Self::create_light_bind_group(
            graphics_device.device(),
            &unidirectional_light_gpu_buffer,
            unidirectional_light_bind_group_layout,
            "Unidirectional light bind group",
        );

        let omnidirectional_light_shadow_map_manager = OmnidirectionalLightShadowMapManager::new(
            graphics_device,
            config,
            omnidirectional_light_gpu_buffer.light_ids().len(),
        );

        let unidirectional_light_shadow_map_manager = UnidirectionalLightShadowMapManager::new(
            graphics_device,
            config,
            unidirectional_light_gpu_buffer.light_ids().len(),
        );

        Self {
            ambient_light_gpu_buffer,
            omnidirectional_light_gpu_buffer,
            unidirectional_light_gpu_buffer,
            ambient_light_bind_group,
            omnidirectional_light_bind_group,
            unidirectional_light_bind_group,
            omnidirectional_light_shadow_map_manager,
            unidirectional_light_shadow_map_manager,
        }
    }

    /// Returns the slice of IDs of all the [`AmbientLight`]s currently residing
    /// in the ambient light GPU buffer.
    pub fn ambient_light_ids(&self) -> &[LightID] {
        self.ambient_light_gpu_buffer.light_ids()
    }

    /// Returns the slice of IDs of all the [`OmnidirectionalLight`]s currently
    /// residing in the omnidirectional light GPU buffer.
    pub fn omnidirectional_light_ids(&self) -> &[LightID] {
        self.omnidirectional_light_gpu_buffer.light_ids()
    }

    /// Returns the slice of IDs of all the [`UnidirectionalLight`]s currently
    /// residing in the unidirectional light GPU buffer.
    pub fn unidirectional_light_ids(&self) -> &[LightID] {
        self.unidirectional_light_gpu_buffer.light_ids()
    }

    /// Returns a reference to the bind group for the ambient light uniform
    /// buffer.
    pub fn ambient_light_bind_group(&self) -> &wgpu::BindGroup {
        &self.ambient_light_bind_group
    }

    /// Returns a reference to the bind group for the omnidirectional light
    /// uniform buffer.
    pub fn omnidirectional_light_bind_group(&self) -> &wgpu::BindGroup {
        &self.omnidirectional_light_bind_group
    }

    /// Returns a reference to the bind group for the unidirectional light
    /// uniform buffer.
    pub fn unidirectional_light_bind_group(&self) -> &wgpu::BindGroup {
        &self.unidirectional_light_bind_group
    }

    /// Returns a reference to the manager for the the omnidirectional light
    /// shadow maps.
    pub fn omnidirectional_light_shadow_map_manager(
        &self,
    ) -> &OmnidirectionalLightShadowMapManager {
        &self.omnidirectional_light_shadow_map_manager
    }

    /// Returns a reference to the manager for the the unidirectional light
    /// shadow maps.
    pub fn unidirectional_light_shadow_map_manager(&self) -> &UnidirectionalLightShadowMapManager {
        &self.unidirectional_light_shadow_map_manager
    }

    /// Returns the bind group for the uniform buffer for the given
    /// light type.
    pub fn bind_group_for_light_type(&self, light_type: LightType) -> &wgpu::BindGroup {
        match light_type {
            LightType::AmbientLight => &self.ambient_light_bind_group,
            LightType::OmnidirectionalLight => &self.omnidirectional_light_bind_group,
            LightType::UnidirectionalLight => &self.unidirectional_light_bind_group,
        }
    }

    /// Returns the current capacity of the ambient light uniform buffer.
    pub fn max_ambient_light_count(&self) -> usize {
        self.ambient_light_gpu_buffer.buffer().max_uniform_count()
    }

    /// Returns the current capacity of the omnidirectional light uniform
    /// buffer.
    pub fn max_omnidirectional_light_count(&self) -> usize {
        self.omnidirectional_light_gpu_buffer
            .buffer()
            .max_uniform_count()
    }

    /// Returns the current capacity of the unidirectional light uniform buffer.
    pub fn max_unidirectional_light_count(&self) -> usize {
        self.unidirectional_light_gpu_buffer
            .buffer()
            .max_uniform_count()
    }

    /// Returns the size of the push constant obtained by calling
    /// [`Self::light_idx_push_constant`].
    pub const fn light_idx_push_constant_size() -> u32 {
        mem::size_of::<u32>() as u32
    }

    /// Finds and returns the index of the light with the given ID in the light
    /// type's uniform buffer, for use as a push constant.
    ///
    /// # Panics
    /// If no light with the given ID is present in the relevant uniform buffer.
    pub fn light_idx_push_constant(&self, light_type: LightType, light_id: LightID) -> u32 {
        let light_idx = match light_type {
            LightType::AmbientLight => &self.ambient_light_gpu_buffer,
            LightType::OmnidirectionalLight => &self.omnidirectional_light_gpu_buffer,
            LightType::UnidirectionalLight => &self.unidirectional_light_gpu_buffer,
        }
        .find_idx_of_light_with_id(light_id)
        .expect("Tried to set light index push constant for missing light");

        u32::try_from(light_idx).unwrap()
    }

    /// Returns the size of the push constant obtained holding a [`CascadeIdx`].
    pub const fn cascade_idx_push_constant_size() -> u32 {
        mem::size_of::<CascadeIdx>() as u32
    }

    /// Ensures that the light uniform buffers are in sync with the light data
    /// in the given light storage. Will also recreate the bind group and update
    /// the shader input if any of the GPU buffers had to be reallocated.
    pub fn sync_with_light_storage(
        &mut self,
        graphics_device: &GraphicsDevice,
        light_storage: &LightStorage,
    ) {
        let omnidirectional_light_buffer_change =
            light_storage.omnidirectional_light_buffer().change();
        let unidirectional_light_buffer_change =
            light_storage.unidirectional_light_buffer().change();

        let ambient_light_transfer_result = self
            .ambient_light_gpu_buffer
            .transfer_uniforms_to_gpu_buffer(graphics_device, light_storage.ambient_light_buffer());

        let omnidirectional_light_transfer_result = self
            .omnidirectional_light_gpu_buffer
            .transfer_uniforms_to_gpu_buffer(
                graphics_device,
                light_storage.omnidirectional_light_buffer(),
            );

        let unidirectional_light_transfer_result = self
            .unidirectional_light_gpu_buffer
            .transfer_uniforms_to_gpu_buffer(
                graphics_device,
                light_storage.unidirectional_light_buffer(),
            );

        if ambient_light_transfer_result == UniformTransferResult::CreatedNewBuffer {
            self.ambient_light_bind_group = Self::create_light_bind_group(
                graphics_device.device(),
                &self.ambient_light_gpu_buffer,
                Self::get_or_create_ambient_light_bind_group_layout(graphics_device),
                "Ambient light bind group",
            );
        }

        if omnidirectional_light_transfer_result == UniformTransferResult::CreatedNewBuffer {
            self.omnidirectional_light_bind_group = Self::create_light_bind_group(
                graphics_device.device(),
                &self.omnidirectional_light_gpu_buffer,
                Self::get_or_create_omnidirectional_light_bind_group_layout(graphics_device),
                "Omnidirectional light bind group",
            );
        }

        if unidirectional_light_transfer_result == UniformTransferResult::CreatedNewBuffer {
            self.unidirectional_light_bind_group = Self::create_light_bind_group(
                graphics_device.device(),
                &self.unidirectional_light_gpu_buffer,
                Self::get_or_create_unidirectional_light_bind_group_layout(graphics_device),
                "Unidirectional light bind group",
            );
        }

        if omnidirectional_light_buffer_change == CollectionChange::Count {
            self.omnidirectional_light_shadow_map_manager
                .create_new_textures_if_required(
                    graphics_device,
                    self.omnidirectional_light_gpu_buffer.light_ids().len(),
                );
        }

        if unidirectional_light_buffer_change == CollectionChange::Count {
            self.unidirectional_light_shadow_map_manager
                .create_new_textures_if_required(
                    graphics_device,
                    self.unidirectional_light_gpu_buffer.light_ids().len(),
                );
        }
    }

    /// Returns the bind group layout for the ambient light uniform buffer,
    /// or creates it if it has not already been created.
    pub fn get_or_create_ambient_light_bind_group_layout(
        graphics_device: &GraphicsDevice,
    ) -> &wgpu::BindGroupLayout {
        AMBIENT_LIGHT_BIND_GROUP_LAYOUT
            .get_or_init(|| Self::create_ambient_light_bind_group_layout(graphics_device.device()))
    }

    /// Returns the bind group layout for the omnidirectional light uniform
    /// buffer, or creates it if it has not already been created.
    pub fn get_or_create_omnidirectional_light_bind_group_layout(
        graphics_device: &GraphicsDevice,
    ) -> &wgpu::BindGroupLayout {
        OMNIDIRECTIONAL_LIGHT_BIND_GROUP_LAYOUT.get_or_init(|| {
            Self::create_omnidirectional_light_bind_group_layout(graphics_device.device())
        })
    }

    /// Returns the bind group layout for the unidirectional light uniform
    /// buffer, or creates it if it has not already been created.
    pub fn get_or_create_unidirectional_light_bind_group_layout(
        graphics_device: &GraphicsDevice,
    ) -> &wgpu::BindGroupLayout {
        UNIDIRECTIONAL_LIGHT_BIND_GROUP_LAYOUT.get_or_init(|| {
            Self::create_unidirectional_light_bind_group_layout(graphics_device.device())
        })
    }

    fn create_ambient_light_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[AmbientLight::create_bind_group_layout_entry(
                Self::light_binding(),
                Self::AMBIENT_LIGHT_VISIBILITY,
            )],
            label: Some("Ambient light bind group layout"),
        })
    }

    fn create_omnidirectional_light_bind_group_layout(
        device: &wgpu::Device,
    ) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[OmnidirectionalLight::create_bind_group_layout_entry(
                Self::light_binding(),
                Self::OMNIDIRECTIONAL_LIGHT_VISIBILITY,
            )],
            label: Some("Omnidirectional light bind group layout"),
        })
    }

    fn create_unidirectional_light_bind_group_layout(
        device: &wgpu::Device,
    ) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[UnidirectionalLight::create_bind_group_layout_entry(
                Self::light_binding(),
                Self::UNIDIRECTIONAL_LIGHT_VISIBILITY,
            )],
            label: Some("Unidirectional light bind group layout"),
        })
    }

    fn create_light_bind_group(
        device: &wgpu::Device,
        light_gpu_buffer: &UniformGPUBufferWithLightIDs,
        layout: &wgpu::BindGroupLayout,
        label: &str,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[light_gpu_buffer
                .buffer()
                .create_bind_group_entry(Self::light_binding())],
            label: Some(label),
        })
    }
}

impl OmnidirectionalLightShadowMapManager {
    fn new(
        graphics_device: &GraphicsDevice,
        config: &RenderingConfig,
        omnidirectional_light_count: usize,
    ) -> Self {
        let resolution = config.omnidirectional_light_shadow_map_resolution;

        let mut manager = Self {
            resolution,
            textures: Vec::with_capacity(omnidirectional_light_count),
            light_count: omnidirectional_light_count,
        };
        manager.create_new_textures_if_required(graphics_device, omnidirectional_light_count);

        manager
    }

    /// Returns the slice of [`ShadowCubemapTexture`]s. The maps are not
    /// inherently associated with any particular light, but the slice is
    /// guaranteed to contain exactly one texture per omnidirectional light.
    pub fn textures(&self) -> &[ShadowCubemapTexture] {
        &self.textures[..self.light_count]
    }

    /// Returns the bind group layout for the shadow cubemap texture and
    /// samplers, or creates it if it has not already been created.
    pub fn get_or_create_bind_group_layout(
        graphics_device: &GraphicsDevice,
    ) -> &wgpu::BindGroupLayout {
        OMNIDIRECTIONAL_LIGHT_SHADOW_MAP_BIND_GROUP_LAYOUT.get_or_init(|| {
            ShadowCubemapTexture::create_bind_group_layout(graphics_device.device())
        })
    }

    fn create_new_textures_if_required(
        &mut self,
        graphics_device: &GraphicsDevice,
        light_count: usize,
    ) {
        if self.textures.len() < light_count {
            let n_additional = light_count - self.textures.len();

            self.textures.reserve(n_additional);
            for _ in 0..n_additional {
                self.textures
                    .push(Self::create_texture(graphics_device, self.resolution));
            }
        }
        self.light_count = light_count;
    }

    fn create_texture(graphics_device: &GraphicsDevice, resolution: u32) -> ShadowCubemapTexture {
        ShadowCubemapTexture::new(
            graphics_device,
            resolution,
            "Omnidirectional light shadow cubemap texture",
        )
    }
}

impl UnidirectionalLightShadowMapManager {
    fn new(
        graphics_device: &GraphicsDevice,
        config: &RenderingConfig,
        unidirectional_light_count: usize,
    ) -> Self {
        let resolution = config.unidirectional_light_shadow_map_resolution;

        let mut manager = Self {
            resolution,
            textures: Vec::with_capacity(unidirectional_light_count),
            light_count: unidirectional_light_count,
        };
        manager.create_new_textures_if_required(graphics_device, unidirectional_light_count);

        manager
    }

    /// Returns the slice of [`CascadedShadowMapTexture`]s. The maps are not
    /// inherently associated with any particular light, but the slice is
    /// guaranteed to contain exactly one texture per unidirectional light.
    pub fn textures(&self) -> &[CascadedShadowMapTexture] {
        &self.textures[..self.light_count]
    }

    /// Returns the bind group layout for the cascaded shadow map texture and
    /// samplers, or creates it if it has not already been created.
    pub fn get_or_create_bind_group_layout(
        graphics_device: &GraphicsDevice,
    ) -> &wgpu::BindGroupLayout {
        UNIDIRECTIONAL_LIGHT_SHADOW_MAP_BIND_GROUP_LAYOUT.get_or_init(|| {
            CascadedShadowMapTexture::create_bind_group_layout(graphics_device.device())
        })
    }

    fn create_new_textures_if_required(
        &mut self,
        graphics_device: &GraphicsDevice,
        light_count: usize,
    ) {
        if self.textures.len() < light_count {
            let n_additional = light_count - self.textures.len();

            self.textures.reserve(n_additional);
            for _ in 0..n_additional {
                self.textures
                    .push(Self::create_texture(graphics_device, self.resolution));
            }
        }
        self.light_count = light_count;
    }

    fn create_texture(
        graphics_device: &GraphicsDevice,
        resolution: u32,
    ) -> CascadedShadowMapTexture {
        CascadedShadowMapTexture::new(
            graphics_device,
            resolution,
            MAX_SHADOW_MAP_CASCADES,
            "Unidirectional light cascaded shadow map texture",
        )
    }
}

impl UniformGPUBufferWithLightIDs {
    /// Creates a new uniform GPU buffer together with a list of light IDs
    /// initialized from the given uniform buffer.
    fn for_uniform_buffer<U>(
        graphics_device: &GraphicsDevice,
        uniform_buffer: &UniformBuffer<LightID, U>,
        visibility: wgpu::ShaderStages,
    ) -> Self
    where
        U: UniformBufferable,
    {
        Self {
            uniform_gpu_buffer: MultiUniformGPUBuffer::for_uniform_buffer(
                graphics_device,
                uniform_buffer,
                visibility,
            ),
            light_ids: uniform_buffer.valid_uniform_ids().to_vec(),
        }
    }

    fn buffer(&self) -> &MultiUniformGPUBuffer {
        &self.uniform_gpu_buffer
    }

    fn light_ids(&self) -> &[LightID] {
        &self.light_ids
    }

    fn find_idx_of_light_with_id(&self, light_id: LightID) -> Option<usize> {
        self.light_ids.iter().position(|&id| id == light_id)
    }

    fn transfer_uniforms_to_gpu_buffer<U>(
        &mut self,
        graphics_device: &GraphicsDevice,
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

        self.uniform_gpu_buffer
            .transfer_uniforms_to_gpu_buffer(graphics_device, uniform_buffer)
    }
}

impl UniformBufferable for AmbientLight {
    const ID: ConstStringHash64 = ConstStringHash64::new("Ambient light");

    fn create_bind_group_layout_entry(
        binding: u32,
        visibility: wgpu::ShaderStages,
    ) -> wgpu::BindGroupLayoutEntry {
        uniform::create_uniform_buffer_bind_group_layout_entry(binding, visibility)
    }
}
assert_uniform_valid!(AmbientLight);

impl UniformBufferable for OmnidirectionalLight {
    const ID: ConstStringHash64 = ConstStringHash64::new("Omnidirectional light");

    fn create_bind_group_layout_entry(
        binding: u32,
        visibility: wgpu::ShaderStages,
    ) -> wgpu::BindGroupLayoutEntry {
        uniform::create_uniform_buffer_bind_group_layout_entry(binding, visibility)
    }
}
assert_uniform_valid!(OmnidirectionalLight);

impl UniformBufferable for UnidirectionalLight {
    const ID: ConstStringHash64 = ConstStringHash64::new("Unidirectional light");

    fn create_bind_group_layout_entry(
        binding: u32,
        visibility: wgpu::ShaderStages,
    ) -> wgpu::BindGroupLayoutEntry {
        uniform::create_uniform_buffer_bind_group_layout_entry(binding, visibility)
    }
}
assert_uniform_valid!(UnidirectionalLight);
