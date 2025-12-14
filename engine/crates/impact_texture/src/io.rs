//! Input/output of texture data.

use crate::processing::ImageProcessing;
use anyhow::{Result, anyhow, bail};
use impact_alloc::{AVec, arena::ArenaPool};
use impact_gpu::{
    device::GraphicsDevice,
    texture::{Texture, TextureConfig, mipmap::MipmapperGenerator},
    wgpu,
};
use impact_io::image;
use std::path::Path;

/// Creates a texture for the image file at the given path, using the given
/// configuration parameters. Mipmaps will be generated automatically.
///
/// # Errors
/// Returns an error if:
/// - The image file can not be read or decoded.
/// - The image bytes can not be interpreted.
/// - The image width or height is zero.
/// - The row size (width times texel size) is not a multiple of 256 bytes
///   (`wgpu` requires that rows are a multiple of 256 bytes for copying
///   data between buffers and textures).
/// - The image is grayscale and the color space in the configuration is not
///   linear.
pub fn create_texture_from_image_path(
    graphics_device: &GraphicsDevice,
    mipmapper_generator: &MipmapperGenerator,
    image_path: impl AsRef<Path>,
    texture_config: TextureConfig,
    processing: &ImageProcessing,
    label: &str,
) -> Result<Texture> {
    let image_path = image_path.as_ref();
    let arena = ArenaPool::get_arena();
    let image = image::load_image_from_path(&arena, image_path)?;
    crate::create_texture_from_image(
        graphics_device,
        mipmapper_generator,
        &image,
        texture_config,
        processing,
        label,
    )
}

/// Saves the texture at the given index of the given texture array as a color
/// or grayscale PNG image at the given output path.
///
/// # Errors
/// Returns an error if:
/// - The format of the given texture is not supported.
/// - The mip level is invalid.
/// - The texture array index is invalid.
pub fn save_texture_as_png_file(
    graphics_device: &GraphicsDevice,
    texture: &wgpu::Texture,
    mip_level: u32,
    texture_array_idx: u32,
    already_gamma_corrected: bool,
    output_path: impl AsRef<Path>,
) -> Result<()> {
    fn byte_to_float(byte: u8) -> f32 {
        f32::from(byte) / 255.0
    }

    fn float_to_byte(float: f32) -> u8 {
        (float.clamp(0.0, 1.0) * 255.0) as u8
    }

    fn linear_to_srgb(linear_value: f32) -> f32 {
        if linear_value <= 0.0031308 {
            linear_value * 12.92
        } else {
            (linear_value.abs().powf(1.0 / 2.4) * 1.055) - 0.055
        }
    }

    fn linear_depth_to_srgb(linear_value: f32) -> f32 {
        // To make small depths darker, we invert before gamma correcting, then
        // invert back
        1.0 - linear_to_srgb(1.0 - linear_value)
    }

    fn linear_byte_to_srgb(linear_value: u8) -> u8 {
        float_to_byte(linear_to_srgb(byte_to_float(linear_value)))
    }

    if mip_level >= texture.mip_level_count() {
        return Err(anyhow!(
            "Mip level {} out of bounds for texture with {} mip levels",
            mip_level,
            texture.mip_level_count()
        ));
    }

    if texture_array_idx >= texture.depth_or_array_layers() {
        return Err(anyhow!(
            "Texture array index {} out of bounds for texture with {} array layers",
            texture_array_idx,
            texture.depth_or_array_layers()
        ));
    }

    let format = texture.format();

    let size = texture
        .size()
        .mip_level_size(mip_level, texture.dimension());

    let arena = ArenaPool::get_arena();

    match format {
        wgpu::TextureFormat::Rgba8Unorm
        | wgpu::TextureFormat::Rgba8UnormSrgb
        | wgpu::TextureFormat::Bgra8Unorm
        | wgpu::TextureFormat::Bgra8UnormSrgb
        | wgpu::TextureFormat::R8Unorm => {
            let mut data = AVec::new_in(&arena);
            impact_gpu::texture::extract_texture_data_into(
                &mut data,
                graphics_device.device(),
                graphics_device.queue(),
                texture,
                mip_level,
                texture_array_idx,
            )?;

            if matches!(
                format,
                wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
            ) {
                convert_bgra8_to_rgba8(&mut data);
            }

            if matches!(format, wgpu::TextureFormat::R8Unorm) {
                if !already_gamma_corrected {
                    for pixel in &mut data {
                        *pixel = linear_byte_to_srgb(*pixel);
                    }
                }

                image::save_luma8_as_png(&data, size.width, size.height, output_path)?;
            } else {
                let (rgba_data, rem) = data.as_chunks_mut::<4>();
                assert!(rem.is_empty());

                if matches!(
                    format,
                    wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm
                ) && !already_gamma_corrected
                {
                    for rgba in rgba_data {
                        rgba[0] = linear_byte_to_srgb(rgba[0]);
                        rgba[1] = linear_byte_to_srgb(rgba[1]);
                        rgba[2] = linear_byte_to_srgb(rgba[2]);
                        rgba[3] = 255;
                    }
                } else {
                    for rgba in rgba_data {
                        rgba[3] = 255;
                    }
                }

                image::save_rgba8_as_png(&data, size.width, size.height, output_path)?;
            }
        }
        wgpu::TextureFormat::Rgba16Float | wgpu::TextureFormat::R16Float => {
            let mut data = AVec::new_in(&arena);
            impact_gpu::texture::extract_converted_texture_data_into::<_, half::f16, f32>(
                &mut data,
                graphics_device.device(),
                graphics_device.queue(),
                texture,
                mip_level,
                texture_array_idx,
            )?;

            if !already_gamma_corrected {
                for value in &mut data {
                    *value = linear_to_srgb(value.clamp(0.0, 1.0));
                }
            }

            if matches!(format, wgpu::TextureFormat::R16Float) {
                let luma8_data: Vec<u8> = data.into_iter().map(float_to_byte).collect();

                impact_io::image::save_luma8_as_png(
                    &luma8_data,
                    size.width,
                    size.height,
                    output_path,
                )?;
            } else {
                let (rgba_f32_data, rem) = data.as_chunks::<4>();
                assert!(rem.is_empty());

                let mut rgba8_data = Vec::with_capacity(data.len());

                for &[r, g, b, _] in rgba_f32_data {
                    rgba8_data.push(float_to_byte(r));
                    rgba8_data.push(float_to_byte(g));
                    rgba8_data.push(float_to_byte(b));
                    rgba8_data.push(255);
                }

                impact_io::image::save_rgba8_as_png(
                    &rgba8_data,
                    size.width,
                    size.height,
                    output_path,
                )?;
            }
        }
        wgpu::TextureFormat::Depth32Float
        | wgpu::TextureFormat::Depth32FloatStencil8
        | wgpu::TextureFormat::Rgba32Float
        | wgpu::TextureFormat::R32Float
        | wgpu::TextureFormat::Rg32Float => {
            let mut data = AVec::<f32, _>::new_in(&arena);
            impact_gpu::texture::extract_texture_data_into(
                &mut data,
                graphics_device.device(),
                graphics_device.queue(),
                texture,
                mip_level,
                texture_array_idx,
            )?;

            if matches!(format, wgpu::TextureFormat::Rg32Float) {
                let (rg_data, rem) = data.as_chunks::<2>();
                assert!(rem.is_empty());

                let mut rgba_data = AVec::new_in(&arena);
                rgba_data.resize(data.len() * 2, 0.0);

                for (i, &[r, g]) in rg_data.iter().enumerate() {
                    rgba_data[i * 4] = r;
                    rgba_data[i * 4 + 1] = g;
                    rgba_data[i * 4 + 2] = 0.0;
                    rgba_data[i * 4 + 3] = 1.0;
                }
                data = rgba_data;
            }

            if matches!(
                format,
                wgpu::TextureFormat::Depth32Float
                    | wgpu::TextureFormat::Depth32FloatStencil8
                    | wgpu::TextureFormat::R32Float
            ) {
                if !already_gamma_corrected {
                    if matches!(format, wgpu::TextureFormat::R32Float) {
                        for value in &mut data {
                            *value = linear_to_srgb(value.clamp(0.0, 1.0));
                        }
                    } else {
                        for value in &mut data {
                            *value = linear_depth_to_srgb(*value);
                        }
                    }
                }

                let luma8_data: Vec<u8> = data.into_iter().map(float_to_byte).collect();

                impact_io::image::save_luma8_as_png(
                    &luma8_data,
                    size.width,
                    size.height,
                    output_path,
                )?;
            } else {
                if !already_gamma_corrected {
                    for value in &mut data {
                        *value = linear_to_srgb(value.clamp(0.0, 1.0));
                    }
                }

                let luma8_data: Vec<u8> = data.into_iter().map(float_to_byte).collect();

                impact_io::image::save_rgba8_as_png(
                    &luma8_data,
                    size.width,
                    size.height,
                    output_path,
                )?;
            }
        }
        _ => {
            bail!(
                "Unsupported texture format for saving as image file: {:?}",
                format
            );
        }
    }

    Ok(())
}

/// Loads and returns the `Bincode` serialized metadata header of the lookup
/// table at the given path.
///
/// # Errors
/// Returns an error if:
/// - The file cannot be opened or read.
/// - The file does not contain valid `Bincode` serialized metadata.
#[cfg(feature = "bincode")]
pub fn read_lookup_table_metadata_from_file(
    file_path: impl AsRef<Path>,
) -> Result<crate::lookup_table::LookupTableMetadata> {
    use anyhow::Context;
    use std::{fs::File, io::BufReader};

    let file_path = file_path.as_ref();

    impact_log::debug!(
        "Reading metadata for lookup table at {}",
        file_path.display()
    );

    let file = File::open(file_path).with_context(|| {
        format!(
            "Failed to open texture lookup table at {}",
            file_path.display()
        )
    })?;
    let reader = BufReader::new(file);
    let metadata = bincode::serde::decode_from_reader(reader, bincode::config::standard())?;
    Ok(metadata)
}

/// Loads and returns the `Bincode` serialized lookup table at the given path.
///
/// # Errors
/// Returns an error if:
/// - The file cannot be opened or read.
/// - The file does not contain valid `Bincode` serialized lookup table data.
#[cfg(feature = "bincode")]
pub fn read_lookup_table_from_file<T>(
    file_path: impl AsRef<Path>,
) -> Result<crate::lookup_table::LookupTable<T>>
where
    T: impact_gpu::texture::TexelType + serde::de::DeserializeOwned,
{
    use anyhow::Context;
    use std::{
        fs::File,
        io::{BufReader, Read},
    };

    let file_path = file_path.as_ref();

    impact_log::debug!("Reading lookup table at {}", file_path.display());

    let file = File::open(file_path).with_context(|| {
        format!(
            "Failed to open texture lookup table at {}",
            file_path.display()
        )
    })?;

    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;

    let (table, _) = bincode::serde::decode_from_slice(&buffer, bincode::config::standard())?;

    Ok(table)
}

/// Serializes a lookup table into the `Bincode` format and saves it at the
/// given path.
///
/// # Errors
/// Returns an error if:
/// - The lookup table cannot be serialized to `Bincode` format.
/// - The serialized data cannot be written to the specified path.
#[cfg(feature = "bincode")]
pub fn save_lookup_table_to_file<T>(
    table: &crate::lookup_table::LookupTable<T>,
    output_file_path: impl AsRef<Path>,
) -> Result<()>
where
    T: impact_gpu::texture::TexelType + serde::Serialize,
{
    let byte_buffer = bincode::serde::encode_to_vec(table, bincode::config::standard())?;
    impact_io::save_data_as_binary(output_file_path, &byte_buffer)?;
    Ok(())
}

fn convert_bgra8_to_rgba8(bgra_bytes: &mut [u8]) {
    for bgra in bgra_bytes.chunks_exact_mut(4) {
        bgra.swap(0, 2);
    }
}
