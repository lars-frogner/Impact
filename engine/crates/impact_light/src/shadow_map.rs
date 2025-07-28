//! Textures representing shadow maps.

use anyhow::Result;
use impact_geometry::CubemapFace;
use impact_gpu::{device::GraphicsDevice, wgpu};
use std::path::Path;

/// Configuration options for shadow mapping.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct ShadowMappingConfig {
    /// Whether shadow mapping is enabled.
    pub enabled: bool,
    /// The width and height of each face of the omnidirectional light shadow
    /// cubemap in number of texels.
    pub omnidirectional_light_shadow_map_resolution: u32,
    /// The width and height of the unidirectional light shadow map in number of
    /// texels.
    pub unidirectional_light_shadow_map_resolution: u32,
}

/// Index representing a cascade in a cascaded shadow map.
pub type CascadeIdx = u32;

pub const SHADOW_MAP_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Float;

/// Texture array for storing the depths of the closest vertices to an
/// omnidirectional light source, used for shadow mapping. Each of the six
/// textures in the array is associated with a face of a cube centered on the
/// light source, and holds the depths in all directions whose dominant
/// component is the outward normal of the cube face.
#[derive(Debug)]
pub struct ShadowCubemapTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    face_views: [wgpu::TextureView; 6],
    sampler: wgpu::Sampler,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

/// Texture array for storing the depths of the closest vertices to a
/// unidirectional light source, used for shadow mapping. Each of the textures
/// in the array stores depths for a separate range of view distances (a
/// partition of the view frustum, referred to as a cascade).
#[derive(Debug)]
pub struct CascadedShadowMapTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    cascade_views: Vec<wgpu::TextureView>,
    sampler: wgpu::Sampler,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl Default for ShadowMappingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            omnidirectional_light_shadow_map_resolution: 1024,
            unidirectional_light_shadow_map_resolution: 1024,
        }
    }
}

impl ShadowCubemapTexture {
    /// The binding location of the shadow cubemap texture.
    pub const fn texture_binding() -> u32 {
        0
    }
    /// The binding location of the shadow cubemap sampler.
    pub const fn sampler_binding() -> u32 {
        1
    }

    /// Creates a new shadow cubemap texture array using the given resolution as
    /// the width and height in texels of each cube face texture.
    pub fn new(graphics_device: &GraphicsDevice, resolution: u32, label: &str) -> Self {
        let device = graphics_device.device();

        let texture_size = wgpu::Extent3d {
            width: resolution,
            height: resolution,
            depth_or_array_layers: 6,
        };

        let texture = create_shadow_map_texture(device, texture_size, label);

        let view = Self::create_view(&texture);

        let face_views = [
            Self::create_face_view(&texture, CubemapFace::PositiveX),
            Self::create_face_view(&texture, CubemapFace::NegativeX),
            Self::create_face_view(&texture, CubemapFace::PositiveY),
            Self::create_face_view(&texture, CubemapFace::NegativeY),
            Self::create_face_view(&texture, CubemapFace::PositiveZ),
            Self::create_face_view(&texture, CubemapFace::NegativeZ),
        ];

        let sampler = create_shadow_map_sampler(device);

        let bind_group_layout = Self::create_bind_group_layout(device);
        let bind_group = Self::create_bind_group(device, &bind_group_layout, &view, &sampler);

        Self {
            texture,
            view,
            face_views,
            sampler,
            bind_group_layout,
            bind_group,
        }
    }

    /// Returns a view into the full shadow cubemap texture.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Returns a view into the given face of the shadow cubemap texture.
    pub fn face_view(&self, face: CubemapFace) -> &wgpu::TextureView {
        &self.face_views[face.as_idx_usize()]
    }

    /// Returns a sampler for the shadow map texture.
    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    /// Returns a reference to the bind group layout for the shadow map texture
    /// and its samplers.
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Returns a reference to the bind group for the shadow map texture
    /// and its samplers.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Saves the specified face texture as a grayscale PNG image at the given
    /// output path.
    pub fn save_face_as_png_file(
        &self,
        graphics_device: &GraphicsDevice,
        face: CubemapFace,
        output_path: impl AsRef<Path>,
    ) -> Result<()> {
        impact_texture::io::save_texture_as_png_file(
            graphics_device,
            &self.texture,
            0,
            face.as_idx_u32(),
            false,
            output_path,
        )
    }

    /// Creates the bind group layout for the shadow cubemap texture and
    /// samplers.
    pub fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                Self::create_texture_bind_group_layout_entry(Self::texture_binding()),
                Self::create_sampler_bind_group_layout_entry(Self::sampler_binding()),
            ],
            label: Some("Shadow cubemap bind group layout"),
        })
    }

    fn create_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        texture_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                Self::create_texture_bind_group_entry(Self::texture_binding(), texture_view),
                Self::create_sampler_bind_group_entry(Self::sampler_binding(), sampler),
            ],
            label: Some("Shadow cubemap bind group"),
        })
    }

    const fn create_texture_bind_group_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                view_dimension: wgpu::TextureViewDimension::Cube,
                multisampled: false,
            },
            count: None,
        }
    }

    const fn create_sampler_bind_group_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
            count: None,
        }
    }

    fn create_texture_bind_group_entry(
        binding: u32,
        texture_view: &wgpu::TextureView,
    ) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(texture_view),
        }
    }

    fn create_sampler_bind_group_entry(
        binding: u32,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Sampler(sampler),
        }
    }

    fn create_view(texture: &wgpu::Texture) -> wgpu::TextureView {
        texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        })
    }

    fn create_face_view(texture: &wgpu::Texture, face: CubemapFace) -> wgpu::TextureView {
        texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2),
            base_array_layer: face.as_idx_u32(),
            array_layer_count: Some(1),
            ..Default::default()
        })
    }
}

impl CascadedShadowMapTexture {
    /// The binding location of the shadow map texture array.
    pub const fn texture_binding() -> u32 {
        0
    }
    /// The binding location of the shadow map sampler.
    pub const fn sampler_binding() -> u32 {
        1
    }

    /// Creates a new cascaded shadow map texture array using the given
    /// resolution as the width and height in texels of each of the `n_cascades`
    /// cascade textures.
    pub fn new(
        graphics_device: &GraphicsDevice,
        resolution: u32,
        n_cascades: u32,
        label: &str,
    ) -> Self {
        assert!(n_cascades > 0);

        let device = graphics_device.device();

        let texture_size = wgpu::Extent3d {
            width: resolution,
            height: resolution,
            depth_or_array_layers: n_cascades,
        };

        let texture = create_shadow_map_texture(device, texture_size, label);

        let view = Self::create_view(&texture);

        let cascade_views = (0..n_cascades)
            .map(|cascade_idx| Self::create_cascade_view(&texture, cascade_idx))
            .collect();

        let sampler = create_shadow_map_sampler(device);

        let bind_group_layout = Self::create_bind_group_layout(device);
        let bind_group = Self::create_bind_group(device, &bind_group_layout, &view, &sampler);

        Self {
            texture,
            view,
            cascade_views,
            sampler,
            bind_group_layout,
            bind_group,
        }
    }

    /// Returns the number of cascades in the shadow map.
    pub fn n_cascades(&self) -> u32 {
        u32::try_from(self.cascade_views.len()).unwrap()
    }

    /// Returns a view into the full cascaded shadow map texture array.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Returns a view into the texture for the given cascade in the shadow map.
    pub fn cascade_view(&self, cascade_idx: CascadeIdx) -> &wgpu::TextureView {
        &self.cascade_views[cascade_idx as usize]
    }

    /// Returns a sampler for the shadow map texture.
    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    /// Returns a reference to the bind group layout for the shadow map texture
    /// and its samplers.
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Returns a reference to the bind group for the shadow map texture
    /// and its samplers.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Saves the specified cascade texture as a grayscale PNG image at the
    /// given output path.
    pub fn save_cascade_as_png_file(
        &self,
        graphics_device: &GraphicsDevice,
        cascade_idx: u32,
        output_path: impl AsRef<Path>,
    ) -> Result<()> {
        impact_texture::io::save_texture_as_png_file(
            graphics_device,
            &self.texture,
            0,
            cascade_idx,
            false,
            output_path,
        )
    }

    /// Creates the bind group layout for the cascaded shadow map texture and
    /// samplers.
    pub fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                Self::create_texture_bind_group_layout_entry(Self::texture_binding()),
                Self::create_sampler_bind_group_layout_entry(Self::sampler_binding()),
            ],
            label: Some("Cascaded shadow map bind group layout"),
        })
    }

    fn create_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        texture_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                Self::create_texture_bind_group_entry(Self::texture_binding(), texture_view),
                Self::create_sampler_bind_group_entry(Self::sampler_binding(), sampler),
            ],
            label: Some("Cascaded shadow map bind group"),
        })
    }

    const fn create_texture_bind_group_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                view_dimension: wgpu::TextureViewDimension::D2Array,
                multisampled: false,
            },
            count: None,
        }
    }

    const fn create_sampler_bind_group_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
            count: None,
        }
    }

    fn create_texture_bind_group_entry(
        binding: u32,
        texture_view: &wgpu::TextureView,
    ) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(texture_view),
        }
    }

    fn create_sampler_bind_group_entry(
        binding: u32,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Sampler(sampler),
        }
    }

    fn create_view(texture: &wgpu::Texture) -> wgpu::TextureView {
        texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        })
    }

    fn create_cascade_view(texture: &wgpu::Texture, cascade_idx: u32) -> wgpu::TextureView {
        texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2),
            base_array_layer: cascade_idx,
            array_layer_count: Some(1),
            ..Default::default()
        })
    }
}

fn create_shadow_map_texture(
    device: &wgpu::Device,
    size: wgpu::Extent3d,
    label: &str,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: SHADOW_MAP_FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC,
        label: Some(label),
        view_formats: &[],
    })
}

fn create_shadow_map_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        ..Default::default()
    })
}
