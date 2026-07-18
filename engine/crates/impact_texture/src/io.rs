//! Input/output of texture data.

use crate::processing::ImageProcessing;
use anyhow::Result;
use impact_alloc::arena::ArenaPool;
use impact_gpu::{
    device::GraphicsDevice,
    texture::{Texture, TextureConfig, mipmap::MipmapperGenerator},
};
use impact_io::image;
use std::path::Path;

/// Colors for rendering texels with non-finite values when saving a texture as
/// an image.
#[cfg(feature = "png")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NonFiniteColors {
    /// RGB color for texels that are NaN.
    pub nan: [u8; 3],
    /// RGB color for texels that are positive infinity.
    pub infinity: [u8; 3],
    /// RGB color for texels that are negative infinity.
    pub neg_infinity: [u8; 3],
}

#[cfg(feature = "png")]
impl Default for NonFiniteColors {
    fn default() -> Self {
        Self {
            nan: [255, 0, 255],
            infinity: [0, 255, 255],
            neg_infinity: [255, 255, 0],
        }
    }
}

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
#[cfg(feature = "png")]
pub fn save_texture_as_png_file(
    graphics_device: &GraphicsDevice,
    texture: &impact_gpu::wgpu::Texture,
    mip_level: u32,
    texture_array_idx: u32,
    already_gamma_corrected: bool,
    non_finite_colors: Option<NonFiniteColors>,
    output_path: impl AsRef<Path>,
) -> Result<()> {
    use anyhow::{anyhow, bail};
    use impact_gpu::wgpu;

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

    let params = PngSaveParams {
        graphics_device,
        texture,
        format,
        mip_level,
        texture_array_idx,
        already_gamma_corrected,
        non_finite_colors,
        width: size.width,
        height: size.height,
        output_path,
    };

    match format {
        wgpu::TextureFormat::Rgba8Unorm
        | wgpu::TextureFormat::Rgba8UnormSrgb
        | wgpu::TextureFormat::Bgra8Unorm
        | wgpu::TextureFormat::Bgra8UnormSrgb
        | wgpu::TextureFormat::R8Unorm => save_u8_texture_as_png(params),
        wgpu::TextureFormat::Rgba16Float | wgpu::TextureFormat::R16Float => {
            save_f16_texture_as_png(params)
        }
        wgpu::TextureFormat::Depth32Float
        | wgpu::TextureFormat::Depth32FloatStencil8
        | wgpu::TextureFormat::Rgba32Float
        | wgpu::TextureFormat::R32Float
        | wgpu::TextureFormat::Rg32Float => save_f32_texture_as_png(params),
        _ => bail!(
            "Unsupported texture format for saving as image file: {:?}",
            format
        ),
    }
}

#[cfg(feature = "png")]
struct PngSaveParams<'a, P: AsRef<Path>> {
    graphics_device: &'a GraphicsDevice,
    texture: &'a impact_gpu::wgpu::Texture,
    format: impact_gpu::wgpu::TextureFormat,
    mip_level: u32,
    texture_array_idx: u32,
    already_gamma_corrected: bool,
    non_finite_colors: Option<NonFiniteColors>,
    width: u32,
    height: u32,
    output_path: P,
}

#[cfg(feature = "png")]
fn byte_to_float(byte: u8) -> f32 {
    f32::from(byte) / 255.0
}

#[cfg(feature = "png")]
fn float_to_byte(float: f32) -> u8 {
    (float.clamp(0.0, 1.0) * 255.0) as u8
}

#[cfg(feature = "png")]
fn linear_to_srgb(linear_value: f32) -> f32 {
    if linear_value <= 0.0031308 {
        linear_value * 12.92
    } else {
        (linear_value.abs().powf(1.0 / 2.4) * 1.055) - 0.055
    }
}

#[cfg(feature = "png")]
fn linear_depth_to_srgb(linear_value: f32) -> f32 {
    // To make small depths darker, we invert before gamma correcting, then
    // invert back
    1.0 - linear_to_srgb(1.0 - linear_value)
}

#[cfg(feature = "png")]
fn linear_byte_to_srgb(linear_value: u8) -> u8 {
    float_to_byte(linear_to_srgb(byte_to_float(linear_value)))
}

#[cfg(feature = "png")]
fn gamma_correct_linear_in_place(data: &mut [f32]) {
    for value in data {
        *value = linear_to_srgb(value.clamp(0.0, 1.0));
    }
}

#[cfg(feature = "png")]
fn save_u8_texture_as_png(params: PngSaveParams<'_, impl AsRef<Path>>) -> Result<()> {
    use impact_alloc::AVec;
    use impact_gpu::wgpu;

    let arena = ArenaPool::get_arena();

    let mut data = AVec::<u8, _>::new_in(&arena);

    impact_gpu::texture::extract_texture_data_into(
        &mut data,
        params.graphics_device.device(),
        params.graphics_device.queue(),
        params.texture,
        params.mip_level,
        params.texture_array_idx,
    )?;

    if matches!(
        params.format,
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
    ) {
        convert_bgra8_to_rgba8(&mut data);
    }

    if matches!(params.format, wgpu::TextureFormat::R8Unorm) {
        if !params.already_gamma_corrected {
            for pixel in &mut data {
                *pixel = linear_byte_to_srgb(*pixel);
            }
        }
        image::save_luma8_as_png(&data, params.width, params.height, params.output_path)
    } else {
        let (rgba_data, rem) = data.as_chunks_mut::<4>();
        assert!(rem.is_empty());

        let gamma_correct = !params.already_gamma_corrected
            && matches!(
                params.format,
                wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm
            );

        if gamma_correct {
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

        image::save_rgba8_as_png(&data, params.width, params.height, params.output_path)
    }
}

#[cfg(feature = "png")]
fn save_f16_texture_as_png(params: PngSaveParams<'_, impl AsRef<Path>>) -> Result<()> {
    use impact_alloc::AVec;
    use impact_gpu::wgpu;

    let arena = ArenaPool::get_arena();

    let mut data = AVec::<f32, _>::new_in(&arena);

    impact_gpu::texture::extract_converted_texture_data_into::<_, half::f16, f32>(
        &mut data,
        params.graphics_device.device(),
        params.graphics_device.queue(),
        params.texture,
        params.mip_level,
        params.texture_array_idx,
    )?;

    if let Some(colors) = params.non_finite_colors {
        let components_per_texel = if matches!(params.format, wgpu::TextureFormat::R16Float) {
            1
        } else {
            4
        };
        let rgba8_data = convert_f32_to_rgba8_with_non_finite_colors(
            &arena,
            &data,
            components_per_texel,
            params.already_gamma_corrected,
            GammaCorrection::Standard,
            colors,
        );
        image::save_rgba8_as_png(&rgba8_data, params.width, params.height, params.output_path)
    } else {
        if !params.already_gamma_corrected {
            gamma_correct_linear_in_place(&mut data);
        }

        if matches!(params.format, wgpu::TextureFormat::R16Float) {
            let mut luma8_data = AVec::with_capacity_in(data.len(), &arena);
            luma8_data.extend(data.into_iter().map(float_to_byte));
            image::save_luma8_as_png(&luma8_data, params.width, params.height, params.output_path)
        } else {
            let (rgba_f32_data, rem) = data.as_chunks::<4>();
            assert!(rem.is_empty());

            let mut rgba8_data = AVec::with_capacity_in(data.len(), &arena);
            for &[r, g, b, _] in rgba_f32_data {
                rgba8_data.push(float_to_byte(r));
                rgba8_data.push(float_to_byte(g));
                rgba8_data.push(float_to_byte(b));
                rgba8_data.push(255);
            }

            image::save_rgba8_as_png(&rgba8_data, params.width, params.height, params.output_path)
        }
    }
}

#[cfg(feature = "png")]
fn save_f32_texture_as_png(params: PngSaveParams<'_, impl AsRef<Path>>) -> Result<()> {
    use impact_alloc::{AVec, avec};
    use impact_gpu::wgpu;

    let arena = ArenaPool::get_arena();

    let mut data = AVec::<f32, _>::new_in(&arena);

    impact_gpu::texture::extract_texture_data_into(
        &mut data,
        params.graphics_device.device(),
        params.graphics_device.queue(),
        params.texture,
        params.mip_level,
        params.texture_array_idx,
    )?;

    // Expand two-channel data to four channels so it can be saved as RGBA.
    if matches!(params.format, wgpu::TextureFormat::Rg32Float) {
        let (rg_data, rem) = data.as_chunks::<2>();
        assert!(rem.is_empty());

        let mut rgba_data = avec![in &arena; 0.0; data.len() * 2];
        for (i, &[r, g]) in rg_data.iter().enumerate() {
            rgba_data[i * 4] = r;
            rgba_data[i * 4 + 1] = g;
            rgba_data[i * 4 + 2] = 0.0;
            rgba_data[i * 4 + 3] = 1.0;
        }
        data = rgba_data;
    }

    let is_depth = matches!(
        params.format,
        wgpu::TextureFormat::Depth32Float | wgpu::TextureFormat::Depth32FloatStencil8
    );

    let is_single_channel = is_depth || matches!(params.format, wgpu::TextureFormat::R32Float);

    if let Some(colors) = params.non_finite_colors {
        let components_per_texel = if is_single_channel { 1 } else { 4 };
        let gamma_correction = if is_depth {
            GammaCorrection::Depth
        } else {
            GammaCorrection::Standard
        };
        let rgba8_data = convert_f32_to_rgba8_with_non_finite_colors(
            &arena,
            &data,
            components_per_texel,
            params.already_gamma_corrected,
            gamma_correction,
            colors,
        );
        image::save_rgba8_as_png(&rgba8_data, params.width, params.height, params.output_path)
    } else {
        if is_single_channel {
            if !params.already_gamma_corrected {
                if matches!(params.format, wgpu::TextureFormat::R32Float) {
                    gamma_correct_linear_in_place(&mut data);
                } else {
                    for value in &mut data {
                        *value = linear_depth_to_srgb(*value);
                    }
                }
            }

            let mut luma8_data = AVec::with_capacity_in(data.len(), &arena);
            luma8_data.extend(data.into_iter().map(float_to_byte));
            image::save_luma8_as_png(&luma8_data, params.width, params.height, params.output_path)
        } else {
            if !params.already_gamma_corrected {
                gamma_correct_linear_in_place(&mut data);
            }
            let mut rgba8_data = AVec::with_capacity_in(data.len(), &arena);
            rgba8_data.extend(data.into_iter().map(float_to_byte));
            image::save_rgba8_as_png(&rgba8_data, params.width, params.height, params.output_path)
        }
    }
}

#[cfg(feature = "png")]
#[derive(Clone, Copy)]
enum GammaCorrection {
    Standard,
    Depth,
}

#[cfg(feature = "png")]
fn convert_f32_to_rgba8_with_non_finite_colors<A>(
    alloc: A,
    data: &[f32],
    components_per_texel: usize,
    already_gamma_corrected: bool,
    gamma_correction: GammaCorrection,
    colors: NonFiniteColors,
) -> impact_alloc::AVec<u8, A>
where
    A: impact_alloc::Allocator,
{
    use impact_alloc::AVec;

    assert!(components_per_texel == 1 || components_per_texel == 4);

    let convert = |mut value: f32| {
        if !already_gamma_corrected {
            value = match gamma_correction {
                GammaCorrection::Standard => linear_to_srgb(value.clamp(0.0, 1.0)),
                GammaCorrection::Depth => linear_depth_to_srgb(value),
            };
        }
        float_to_byte(value)
    };

    let texel_count = data.len() / components_per_texel;
    let mut rgba8_data = AVec::with_capacity_in(texel_count * 4, alloc);

    for texel in data.chunks_exact(components_per_texel) {
        let [r, g, b] = if texel.iter().any(|value| value.is_nan()) {
            colors.nan
        } else if texel.contains(&f32::INFINITY) {
            colors.infinity
        } else if texel.contains(&f32::NEG_INFINITY) {
            colors.neg_infinity
        } else if components_per_texel == 1 {
            let value = convert(texel[0]);
            [value, value, value]
        } else {
            [convert(texel[0]), convert(texel[1]), convert(texel[2])]
        };
        rgba8_data.extend_from_slice(&[r, g, b, 255]);
    }

    rgba8_data
}

/// Loads and returns the `Postcard` serialized metadata header of the lookup
/// table at the given path.
///
/// # Errors
/// Returns an error if:
/// - The file cannot be opened or read.
/// - The file does not contain valid `Postcard` serialized metadata.
#[cfg(feature = "postcard")]
pub fn read_lookup_table_metadata_from_file(
    file_path: impl AsRef<Path>,
) -> Result<crate::lookup_table::LookupTableMetadata> {
    use anyhow::Context;
    use std::{
        fs::File,
        io::{BufReader, Read},
    };

    let file_path = file_path.as_ref();

    log::debug!(
        "Reading metadata for lookup table at {}",
        file_path.display()
    );

    let file = File::open(file_path).with_context(|| {
        format!(
            "Failed to open texture lookup table at {}",
            file_path.display()
        )
    })?;
    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    let metadata = postcard::from_bytes(&buffer)?;
    Ok(metadata)
}

/// Loads and returns the `Postcard` serialized lookup table at the given path.
///
/// # Errors
/// Returns an error if:
/// - The file cannot be opened or read.
/// - The file does not contain valid `Postcard` serialized lookup table data.
#[cfg(feature = "postcard")]
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

    log::debug!("Reading lookup table at {}", file_path.display());

    let file = File::open(file_path).with_context(|| {
        format!(
            "Failed to open texture lookup table at {}",
            file_path.display()
        )
    })?;

    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;

    let table = postcard::from_bytes(&buffer)?;

    Ok(table)
}

/// Serializes a lookup table into the `Postcard` format and saves it at the
/// given path.
///
/// # Errors
/// Returns an error if:
/// - The lookup table cannot be serialized to `Postcard` format.
/// - The serialized data cannot be written to the specified path.
#[cfg(feature = "postcard")]
pub fn save_lookup_table_to_file<T>(
    table: &crate::lookup_table::LookupTable<T>,
    output_file_path: impl AsRef<Path>,
) -> Result<()>
where
    T: impact_gpu::texture::TexelType + serde::Serialize,
{
    let byte_buffer = postcard::to_allocvec(table)?;
    impact_io::save_data_as_binary(output_file_path, &byte_buffer)?;
    Ok(())
}

#[allow(dead_code)]
fn convert_bgra8_to_rgba8(bgra_bytes: &mut [u8]) {
    for bgra in bgra_bytes.chunks_exact_mut(4) {
        bgra.swap(0, 2);
    }
}
