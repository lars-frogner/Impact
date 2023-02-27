//! Textures.

mod depth;
mod image;
mod shadow_map;

pub use self::image::ImageTexture;
pub use depth::DepthTexture;
pub use shadow_map::ShadowMapTexture;

use crate::rendering::CoreRenderingSystem;
use ::image::{buffer::ConvertBuffer, ImageBuffer, Luma, Rgba};
use anyhow::{bail, Result};
use bytemuck::Pod;
use std::{num::NonZeroU32, path::Path};
use wgpu::util::DeviceExt;

/// Saves the given color texture as a color image at the given output path. The
/// image file format is automatically determined from the file extension.
///
/// Supported texture formats are RGBA8 and BGRA8.
///
/// # Errors
/// Returns an error if the format of the given texture is not supported.
pub fn save_color_texture_as_image_file<P: AsRef<Path>>(
    core_system: &CoreRenderingSystem,
    texture: &wgpu::Texture,
    output_path: P,
) -> Result<()> {
    let mut data = extract_texture_data(core_system.device(), core_system.queue(), texture);

    match texture.format() {
        wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => {}
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => {
            convert_bgra8_to_rgba8(&mut data);
        }
        format => {
            bail!(
                "Unsupported texture format for saving as color image file: {:?}",
                format
            );
        }
    }

    let image_buffer =
        ImageBuffer::<Rgba<u8>, _>::from_raw(texture.width(), texture.height(), data).unwrap();

    image_buffer.save(output_path)?;

    Ok(())
}

/// Saves the given depth texture as a grayscale image at the given output path.
/// The image file format is automatically determined from the file extension.
///
/// The supported texture format is [`wgpu::TextureFormat::Depth32Float`].
///
/// # Errors
/// Returns an error if the format of the given texture is not supported.
pub fn save_depth_texture_as_image_file<P: AsRef<Path>>(
    core_system: &CoreRenderingSystem,
    texture: &wgpu::Texture,
    output_path: P,
) -> Result<()> {
    if texture.format() != wgpu::TextureFormat::Depth32Float {
        bail!(
            "Unsupported depth texture format for saving as image file: {:?}",
            texture.format()
        );
    }

    let mut data = extract_texture_data::<f32>(core_system.device(), core_system.queue(), texture);

    // Map from [-1, 1] to [0, 1]
    for val in &mut data {
        *val = 0.5 * (*val + 1.0);
    }

    let image_buffer: ImageBuffer<Luma<f32>, _> =
        ImageBuffer::from_raw(texture.width(), texture.height(), data).unwrap();

    let image_buffer: ImageBuffer<Luma<u16>, _> = image_buffer.convert();

    image_buffer.save(output_path)?;

    Ok(())
}

fn extract_texture_data<T: Pod>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
) -> Vec<T> {
    assert_eq!(texture.depth_or_array_layers(), 1);

    let width = texture.width();
    let height = texture.height();
    let pixel_size = u32::from(texture.format().describe().block_size);

    let mut command_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Texture copy encoder"),
    });

    let raw_buffer = vec![0; (pixel_size * width * height) as usize];

    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        contents: raw_buffer.as_slice(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        label: Some("Texture buffer"),
    });

    command_encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::ImageCopyBuffer {
            buffer: &buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(NonZeroU32::new(pixel_size * width).unwrap()),
                rows_per_image: Some(NonZeroU32::new(height).unwrap()),
            },
        },
        texture.size(),
    );

    queue.submit(std::iter::once(command_encoder.finish()));

    let buffer_slice = buffer.slice(..);
    buffer_slice.map_async(wgpu::MapMode::Read, |result| result.unwrap());
    device.poll(wgpu::Maintain::Wait);
    let buffer_view = buffer_slice.get_mapped_range();

    bytemuck::cast_slice(&buffer_view).to_vec()
}

fn convert_bgra8_to_rgba8(bgra_bytes: &mut [u8]) {
    for bgra in bgra_bytes.chunks_exact_mut(4) {
        bgra.swap(0, 2);
    }
}
