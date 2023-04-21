//! Textures representing shadow maps.

use crate::{geometry::CubemapFace, rendering::CoreRenderingSystem};
use anyhow::Result;
use std::path::Path;

/// Index representing a cascade in a cascaded shadow map.
pub type CascadeIdx = u32;

pub const SHADOW_MAP_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

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
    comparison_sampler: wgpu::Sampler,
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
    comparison_sampler: wgpu::Sampler,
}

impl ShadowCubemapTexture {
    /// Creates a new shadow cubemap texture array using the given resolution as
    /// the width and height in texels of each cube face texture.
    pub fn new(core_system: &CoreRenderingSystem, resolution: u32, label: &str) -> Self {
        let device = core_system.device();

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
        let comparison_sampler = create_shadow_map_comparison_sampler(device);

        Self {
            texture,
            view,
            face_views,
            sampler,
            comparison_sampler,
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

    /// Returns a comparison sampler for the shadow map texture.
    pub fn comparison_sampler(&self) -> &wgpu::Sampler {
        &self.comparison_sampler
    }

    /// Creates the bind group layout entry for this texture type, assigned to
    /// the given binding.
    pub const fn create_texture_bind_group_layout_entry(
        binding: u32,
    ) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Depth,
                view_dimension: wgpu::TextureViewDimension::Cube,
                multisampled: false,
            },
            count: None,
        }
    }

    /// Creates the bind group layout entry for this texture's sampler type,
    /// assigned to the given binding.
    pub const fn create_sampler_bind_group_layout_entry(
        binding: u32,
    ) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
            count: None,
        }
    }

    /// Creates the bind group layout entry for this texture's comparison sampler type,
    /// assigned to the given binding.
    pub const fn create_comparison_sampler_bind_group_layout_entry(
        binding: u32,
    ) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
            count: None,
        }
    }

    /// Creates the bind group entry for this texture, assigned to the given
    /// binding.
    pub fn create_texture_bind_group_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(self.view()),
        }
    }

    /// Creates the bind group entry for this texture's sampler, assigned to the
    /// given binding.
    pub fn create_sampler_bind_group_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Sampler(self.sampler()),
        }
    }

    /// Creates the bind group entry for this texture's comparison sampler, assigned to the
    /// given binding.
    pub fn create_comparison_sampler_bind_group_entry(
        &self,
        binding: u32,
    ) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Sampler(self.comparison_sampler()),
        }
    }

    /// Saves the specified face texture as a grayscale image at the given
    /// output path. The image file format is automatically determined from the
    /// file extension.
    pub fn save_face_as_image_file<P: AsRef<Path>>(
        &self,
        core_system: &CoreRenderingSystem,
        face: CubemapFace,
        output_path: P,
    ) -> Result<()> {
        super::save_depth_texture_as_image_file(
            core_system,
            &self.texture,
            face.as_idx_u32(),
            output_path,
        )
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
    /// Creates a new cascaded shadow map texture array using the given
    /// resolution as the width and height in texels of each of the `n_cascades`
    /// cascade textures.
    pub fn new(
        core_system: &CoreRenderingSystem,
        resolution: u32,
        n_cascades: u32,
        label: &str,
    ) -> Self {
        assert!(n_cascades > 0);

        let device = core_system.device();

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
        let comparison_sampler = create_shadow_map_comparison_sampler(device);

        Self {
            texture,
            view,
            cascade_views,
            sampler,
            comparison_sampler,
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

    /// Returns a comparison sampler for the shadow map texture.
    pub fn comparison_sampler(&self) -> &wgpu::Sampler {
        &self.comparison_sampler
    }

    /// Creates the bind group layout entry for this texture type, assigned to
    /// the given binding.
    pub const fn create_texture_bind_group_layout_entry(
        binding: u32,
    ) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Depth,
                view_dimension: wgpu::TextureViewDimension::D2Array,
                multisampled: false,
            },
            count: None,
        }
    }

    /// Creates the bind group layout entry for this texture's sampler type,
    /// assigned to the given binding.
    pub const fn create_sampler_bind_group_layout_entry(
        binding: u32,
    ) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
            count: None,
        }
    }

    /// Creates the bind group layout entry for this texture's comparison
    /// sampler type, assigned to the given binding.
    pub const fn create_comparison_sampler_bind_group_layout_entry(
        binding: u32,
    ) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
            count: None,
        }
    }

    /// Creates the bind group entry for this texture, assigned to the given
    /// binding.
    pub fn create_texture_bind_group_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(self.view()),
        }
    }

    /// Creates the bind group entry for this texture's sampler, assigned to the
    /// given binding.
    pub fn create_sampler_bind_group_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Sampler(self.sampler()),
        }
    }

    /// Creates the bind group entry for this texture's comparison sampler,
    /// assigned to the given binding.
    pub fn create_comparison_sampler_bind_group_entry(
        &self,
        binding: u32,
    ) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Sampler(self.comparison_sampler()),
        }
    }

    /// Saves the specified cascade texture as a grayscale image at the given
    /// output path. The image file format is automatically determined from the
    /// file extension.
    pub fn save_cascade_as_image_file<P: AsRef<Path>>(
        &self,
        core_system: &CoreRenderingSystem,
        cascade_idx: u32,
        output_path: P,
    ) -> Result<()> {
        super::save_depth_texture_as_image_file(
            core_system,
            &self.texture,
            cascade_idx,
            output_path,
        )
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
        compare: None,
        ..Default::default()
    })
}

fn create_shadow_map_comparison_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        // The result of the comparison sampling will be 1.0 if the
        // reference depth is less than or equal to the sampled depth
        // (meaning that the fragment is not occluded), and 0.0 otherwise.
        compare: Some(wgpu::CompareFunction::LessEqual),
        ..Default::default()
    })
}
