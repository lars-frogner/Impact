//! Input/output of texture data.

use anyhow::{Result, anyhow, bail};
use impact_gpu::{
    device::GraphicsDevice,
    texture::{SamplerID, Texture, TextureConfig, mipmap::MipmapperGenerator},
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
    sampler_id: Option<SamplerID>,
) -> Result<Texture> {
    let image_path = image_path.as_ref();
    let image = image::load_image_from_path(image_path)?;
    crate::create_texture_from_image(
        graphics_device,
        mipmapper_generator,
        image,
        texture_config,
        sampler_id,
        &image_path.to_string_lossy(),
    )
}

/// Creates a cubemap texture for the image files representing cubemap faces
/// at the given paths, using the given configuration parameters.
///
/// # Errors
/// Returns an error if:
/// - The image dimensions or pixel formats do not match.
/// - The image file can not be read or decoded.
/// - The image width or height is zero.
/// - The row size (width times texel size) is not a multiple of 256 bytes
///   (`wgpu` requires that rows are a multiple of 256 bytes for copying
///   data between buffers and textures).
/// - The image is grayscale and the color space in the configuration is not
///   linear.
pub fn create_cubemap_texture_from_image_paths<P: AsRef<Path>>(
    graphics_device: &GraphicsDevice,
    right_image_path: P,
    left_image_path: P,
    top_image_path: P,
    bottom_image_path: P,
    front_image_path: P,
    back_image_path: P,
    texture_config: TextureConfig,
    sampler_id: Option<SamplerID>,
) -> Result<Texture> {
    let right_image_path = right_image_path.as_ref();
    let left_image_path = left_image_path.as_ref();
    let top_image_path = top_image_path.as_ref();
    let bottom_image_path = bottom_image_path.as_ref();
    let front_image_path = front_image_path.as_ref();
    let back_image_path = back_image_path.as_ref();

    let right_image = image::load_image_from_path(right_image_path)?;
    let left_image = image::load_image_from_path(left_image_path)?;
    let top_image = image::load_image_from_path(top_image_path)?;
    let bottom_image = image::load_image_from_path(bottom_image_path)?;
    let front_image = image::load_image_from_path(front_image_path)?;
    let back_image = image::load_image_from_path(back_image_path)?;

    let label = format!(
        "Cubemap {{{}, {}, {}, {}, {}, {}}}",
        right_image_path.display(),
        left_image_path.display(),
        top_image_path.display(),
        bottom_image_path.display(),
        front_image_path.display(),
        back_image_path.display()
    );

    crate::create_cubemap_texture_from_images(
        graphics_device,
        right_image,
        left_image,
        top_image,
        bottom_image,
        front_image,
        back_image,
        texture_config,
        sampler_id,
        &label,
    )
}

/// Creates a texture array for the image files at the given paths, using
/// the given configuration parameters.
///
/// # Errors
/// Returns an error if:
/// - The image file can not be read or decoded.
/// - The number of images is zero.
/// - Any of the images are wrapped in an [`Err`].
/// - The image width or height is zero.
/// - The image is grayscale and the color space in the configuration is not
///   linear.
/// - The image dimensions or pixel formats do not match.
/// - The row size (width times texel size) is not a multiple of 256 bytes
///   (`wgpu` requires that rows are a multiple of 256 bytes for copying
///   data between buffers and textures).
pub fn create_texture_array_from_image_paths<I, P>(
    graphics_device: &GraphicsDevice,
    mipmapper_generator: &MipmapperGenerator,
    image_paths: impl IntoIterator<IntoIter = I>,
    texture_config: TextureConfig,
    sampler_id: Option<SamplerID>,
    label: &str,
) -> Result<Texture>
where
    I: ExactSizeIterator<Item = P>,
    P: AsRef<Path>,
{
    let images = image_paths
        .into_iter()
        .map(|image_path: P| image::load_image_from_path(image_path.as_ref()));

    crate::create_texture_array_from_images(
        graphics_device,
        mipmapper_generator,
        images,
        texture_config,
        sampler_id,
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

    match format {
        wgpu::TextureFormat::Rgba8Unorm
        | wgpu::TextureFormat::Rgba8UnormSrgb
        | wgpu::TextureFormat::Bgra8Unorm
        | wgpu::TextureFormat::Bgra8UnormSrgb
        | wgpu::TextureFormat::R8Unorm => {
            let mut data = impact_gpu::texture::extract_texture_bytes(
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
            let mut data = impact_gpu::texture::extract_texture_data_and_convert::<half::f16, f32>(
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
            let mut data = impact_gpu::texture::extract_texture_data::<f32>(
                graphics_device.device(),
                graphics_device.queue(),
                texture,
                mip_level,
                texture_array_idx,
            )?;

            if matches!(format, wgpu::TextureFormat::Rg32Float) {
                let (rg_data, rem) = data.as_chunks::<2>();
                assert!(rem.is_empty());

                let mut rgba_data = vec![0.0; data.len() * 2];

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

/// Serializes a lookup table into the `Bincode` format and saves it at the
/// given path.
#[cfg(feature = "bincode")]
pub fn save_lookup_table_to_file<T>(
    table: &impact_gpu::texture::TextureLookupTable<T>,
    output_file_path: impl AsRef<Path>,
) -> Result<()>
where
    T: impact_gpu::texture::TexelType + serde::Serialize,
{
    let byte_buffer = bincode::serde::encode_to_vec(table, bincode::config::standard())?;
    impact_io::save_data_as_binary(output_file_path, &byte_buffer)?;
    Ok(())
}

/// Loads and returns the `Bincode` serialized lookup table at the given path.
#[cfg(feature = "bincode")]
pub fn read_lookup_table_from_file<T>(
    file_path: impl AsRef<Path>,
) -> Result<impact_gpu::texture::TextureLookupTable<T>>
where
    T: impact_gpu::texture::TexelType + serde::de::DeserializeOwned,
{
    use anyhow::Context;
    use std::{
        fs::File,
        io::{BufReader, Read},
    };

    let file_path = file_path.as_ref();
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

fn convert_bgra8_to_rgba8(bgra_bytes: &mut [u8]) {
    for bgra in bgra_bytes.chunks_exact_mut(4) {
        bgra.swap(0, 2);
    }
}
