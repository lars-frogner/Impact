//! Poly Haven provider implementation.

use crate::{fetch, providers::AssetDownload};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::io::Read;

/// Asset information specific to Poly Haven provider.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetInfo {
    /// Texture asset (PBR materials)
    Texture(TextureAssetInfo),
    /// 3D model asset
    Model(ModelAssetInfo),
}

/// Information required to fetch a texture from Poly Haven.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextureAssetInfo {
    /// Poly Haven asset ID (e.g., "brick_wall_001", "wood_planks_02").
    pub id: String,
    /// Texture resolution, defaults to 4K.
    #[serde(default)]
    pub resolution: TextureResolution,
    /// Image format, defaults to JPG.
    #[serde(default)]
    pub format: ImageFormat,
    /// Components to download (diffuse, normal, rough, etc.).
    #[serde(default = "default_texture_components")]
    pub components: Vec<TextureAssetComponent>,
}

/// Information required to fetch a 3D model from Poly Haven.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelAssetInfo {
    /// Poly Haven asset ID.
    pub id: String,
    /// Model resolution, defaults to 4K.
    #[serde(default)]
    pub resolution: TextureResolution,
    /// Model format.
    #[serde(default)]
    pub format: ModelFormat,
    /// Image format for texture components.
    pub image_format: ImageFormat,
    /// Components to download (main model file, textures, etc.).
    #[serde(default = "default_model_components")]
    pub components: Vec<ModelAssetComponent>,
}

/// Available texture resolutions on Poly Haven.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextureResolution {
    /// 1024x1024 resolution.
    OneK,
    /// 2048x2048 resolution.
    TwoK,
    /// 4096x4096 resolution (default).
    #[default]
    FourK,
    /// 8192x8192 resolution.
    EightK,
    /// 16384x16384 resolution.
    SixteenK,
}

/// Available texture components on Poly Haven.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextureAssetComponent {
    /// Blender material file.
    Blend,
    /// glTF material file.
    Gltf,
    /// MaterialX file.
    Mtlx,
    /// Ambient occlusion map.
    AO,
    /// Combined AO/Roughness/Metallic map.
    AORoughMetal,
    /// Diffuse/Albedo map.
    Diffuse,
    /// Displacement map.
    Displacement,
    /// Normal map (OpenGL format).
    Normal,
    /// Roughness map.
    Rough,
}

/// Available model components on Poly Haven.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelAssetComponent {
    /// Blender model file.
    Blend,
    /// glTF model file.
    Gltf,
    /// FBX model file.
    Fbx,
    /// USD model file.
    Usd,
    /// Ambient occlusion map.
    AO,
    /// Combined AO/Roughness/Metallic map.
    AORoughMetal,
    /// Diffuse/Albedo map.
    Diffuse,
    /// Displacement map.
    Displacement,
    /// Normal map (DirectX format).
    Normal,
    /// Roughness map.
    Rough,
}

/// Available image formats for textures.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    /// JPEG format (default, good compression).
    #[default]
    Jpg,
    /// PNG format (lossless).
    Png,
    /// OpenEXR format (HDR, 32-bit).
    Exr,
}

/// Available model formats.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelFormat {
    /// glTF format (default).
    #[default]
    Gltf,
    /// Blender format.
    Blend,
    /// FBX format.
    Fbx,
    /// USD format.
    Usd,
}

impl TextureResolution {
    fn as_str(&self) -> &'static str {
        match self {
            Self::OneK => "1k",
            Self::TwoK => "2k",
            Self::FourK => "4k",
            Self::EightK => "8k",
            Self::SixteenK => "16k",
        }
    }
}

impl ImageFormat {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Jpg => "jpg",
            Self::Png => "png",
            Self::Exr => "exr",
        }
    }
}

impl ModelFormat {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Gltf => "gltf",
            Self::Blend => "blend",
            Self::Fbx => "fbx",
            Self::Usd => "usd",
        }
    }
}

impl AssetInfo {
    /// Gets all downloads required for this Poly Haven asset.
    ///
    /// Queries the Poly Haven API to get download URLs for all requested content types.
    pub fn get_downloads(&self) -> Result<Vec<AssetDownload>> {
        match self {
            Self::Texture(info) => self.get_texture_downloads(info),
            Self::Model(info) => self.get_model_downloads(info),
        }
    }

    /// Helper method to extract TextureAssetInfo from AssetInfo for testing.
    #[cfg(test)]
    fn as_texture_info(&self) -> &TextureAssetInfo {
        match self {
            Self::Texture(info) => info,
            Self::Model(_) => panic!("Called as_texture_info on model asset"),
        }
    }

    /// Helper method to extract model downloads with provided model API response for testing.
    #[cfg(test)]
    fn get_model_downloads_with_response(
        &self,
        model_api_response: &ModelApiResponse,
    ) -> Result<Vec<AssetDownload>> {
        match self {
            Self::Model(info) => self.extract_model_from_response(model_api_response, info),
            Self::Texture(_) => panic!("Called get_model_downloads_with_response on texture asset"),
        }
    }

    fn get_texture_downloads(&self, info: &TextureAssetInfo) -> Result<Vec<AssetDownload>> {
        let api_url = format!("https://api.polyhaven.com/files/{}", info.id);

        let mut response = fetch::create_ureq_agent()
            .get(&api_url)
            .header("User-Agent", "ImpactGameEngineAssetFetcher/0.1")
            .call()
            .context("Failed to query Poly Haven API")?;

        let mut body = String::new();
        response
            .body_mut()
            .as_reader()
            .read_to_string(&mut body)
            .context("Failed to read Poly Haven API response")?;

        let api_response: TextureApiResponse =
            serde_json::from_str(&body).context("Failed to parse Poly Haven API response")?;

        let mut downloads = Vec::new();

        for component in &info.components {
            if let Some(download) =
                self.extract_texture_download(&api_response, info, *component)?
            {
                downloads.push(download);
            }
        }

        if downloads.is_empty() {
            bail!(
                "No downloads found for texture {} with the specified components",
                info.id
            );
        }

        Ok(downloads)
    }

    fn get_model_downloads(&self, info: &ModelAssetInfo) -> Result<Vec<AssetDownload>> {
        let api_url = format!("https://api.polyhaven.com/files/{}", info.id);

        let mut response = ureq::get(&api_url)
            .call()
            .context("Failed to query Poly Haven API")?;

        let mut body = String::new();
        response
            .body_mut()
            .as_reader()
            .read_to_string(&mut body)
            .context("Failed to read Poly Haven API response")?;

        let api_response: TextureApiResponse =
            serde_json::from_str(&body).context("Failed to parse Poly Haven API response")?;

        self.extract_model_download(&api_response, info)
    }

    fn extract_texture_download(
        &self,
        api_response: &TextureApiResponse,
        info: &TextureAssetInfo,
        component: TextureAssetComponent,
    ) -> Result<Option<AssetDownload>> {
        let resolution_str = info.resolution.as_str();
        let format_str = info.format.as_str();

        match component {
            TextureAssetComponent::Diffuse => {
                if let Some(diffuse_data) = &api_response.diffuse {
                    self.extract_from_texture_map(
                        diffuse_data,
                        resolution_str,
                        format_str,
                        "diffuse",
                    )
                } else {
                    Ok(None)
                }
            }
            TextureAssetComponent::Normal => {
                if let Some(normal_data) = &api_response.nor_dx {
                    self.extract_from_texture_map(normal_data, resolution_str, format_str, "nor_dx")
                } else {
                    Ok(None)
                }
            }
            TextureAssetComponent::Rough => {
                if let Some(rough_data) = &api_response.rough {
                    self.extract_from_texture_map(rough_data, resolution_str, format_str, "rough")
                } else {
                    Ok(None)
                }
            }
            TextureAssetComponent::AO => {
                if let Some(ao_data) = &api_response.ao {
                    self.extract_from_texture_map(ao_data, resolution_str, format_str, "ao")
                } else {
                    Ok(None)
                }
            }
            TextureAssetComponent::Displacement => {
                if let Some(disp_data) = &api_response.displacement {
                    self.extract_from_texture_map(disp_data, resolution_str, format_str, "disp")
                } else {
                    Ok(None)
                }
            }
            TextureAssetComponent::AORoughMetal => {
                if let Some(arm_data) = &api_response.arm {
                    self.extract_from_texture_map(arm_data, resolution_str, format_str, "arm")
                } else {
                    Ok(None)
                }
            }
            TextureAssetComponent::Blend => {
                if let Some(blend_data) = &api_response.blend {
                    self.extract_from_material_data(blend_data, resolution_str, "blend")
                } else {
                    Ok(None)
                }
            }
            TextureAssetComponent::Gltf => {
                if let Some(gltf_data) = &api_response.gltf {
                    self.extract_from_material_data(gltf_data, resolution_str, "gltf")
                } else {
                    Ok(None)
                }
            }
            TextureAssetComponent::Mtlx => {
                if let Some(mtlx_data) = &api_response.mtlx {
                    self.extract_from_material_data(mtlx_data, resolution_str, "mtlx")
                } else {
                    Ok(None)
                }
            }
        }
    }

    fn extract_from_texture_map(
        &self,
        texture_data: &TextureMapData,
        resolution: &str,
        format: &str,
        content_name: &str,
    ) -> Result<Option<AssetDownload>> {
        // Get the resolution data
        let resolution_data = match resolution {
            "1k" => texture_data.one_k.as_ref(),
            "2k" => texture_data.two_k.as_ref(),
            "4k" => texture_data.four_k.as_ref(),
            "8k" => texture_data.eight_k.as_ref(),
            _ => return Ok(None),
        };

        let resolution_data = match resolution_data {
            Some(data) => data,
            None => return Ok(None),
        };

        // Get the format data
        let file_info = match format {
            "jpg" => resolution_data.jpg.as_ref(),
            "png" => resolution_data.png.as_ref(),
            "exr" => resolution_data.exr.as_ref(),
            _ => return Ok(None),
        };

        let file_info = match file_info {
            Some(info) => info,
            None => return Ok(None),
        };

        // Extract filename from URL
        let default_filename = format!("{content_name}_{resolution}.{format}");
        let filename = file_info
            .url
            .rsplit('/')
            .next()
            .unwrap_or(&default_filename);

        Ok(Some(AssetDownload {
            url: file_info.url.clone(),
            file_path: filename.to_string(),
            size: Some(file_info.size),
            md5: file_info.md5.clone(),
        }))
    }

    fn extract_from_material_data(
        &self,
        material_data: &MaterialData,
        resolution: &str,
        content_name: &str,
    ) -> Result<Option<AssetDownload>> {
        // Get the resolution data
        let resolution_data = match resolution {
            "1k" => material_data.one_k.as_ref(),
            "2k" => material_data.two_k.as_ref(),
            "4k" => material_data.four_k.as_ref(),
            "8k" => material_data.eight_k.as_ref(),
            "16k" => material_data.sixteen_k.as_ref(),
            _ => return Ok(None),
        };

        let resolution_data = match resolution_data {
            Some(data) => data,
            None => return Ok(None),
        };

        // For material files, we typically want the main file (blend, gltf, mtlx)
        let file_info = match content_name {
            "blend" => resolution_data.blend.as_ref(),
            "gltf" => resolution_data.gltf.as_ref(),
            "mtlx" => resolution_data.mtlx.as_ref(),
            _ => return Ok(None),
        };

        let file_info = match file_info {
            Some(info) => info,
            None => return Ok(None),
        };

        // Extract filename from URL
        let default_filename = format!("material_{resolution}.{content_name}");
        let filename = file_info
            .url
            .rsplit('/')
            .next()
            .unwrap_or(&default_filename);

        Ok(Some(AssetDownload {
            url: file_info.url.clone(),
            file_path: filename.to_string(),
            size: Some(file_info.size),
            md5: file_info.md5.clone(),
        }))
    }

    fn extract_model_download(
        &self,
        _api_response: &TextureApiResponse,
        info: &ModelAssetInfo,
    ) -> Result<Vec<AssetDownload>> {
        // For models, we need to make a separate API call to get model-specific data
        // since the texture API response doesn't contain model export data
        let api_url = format!("https://api.polyhaven.com/files/{}", info.id);

        let mut response = fetch::create_ureq_agent()
            .get(&api_url)
            .call()
            .context("Failed to query Poly Haven API for model")?;

        let mut body = String::new();
        response
            .body_mut()
            .as_reader()
            .read_to_string(&mut body)
            .context("Failed to read Poly Haven API response")?;

        let model_api_response: ModelApiResponse =
            serde_json::from_str(&body).context("Failed to parse Poly Haven API response")?;

        self.extract_model_from_response(&model_api_response, info)
    }

    fn extract_model_from_response(
        &self,
        api_response: &ModelApiResponse,
        info: &ModelAssetInfo,
    ) -> Result<Vec<AssetDownload>> {
        let resolution_str = info.resolution.as_str();
        let format_str = info.format.as_str();

        // Get the appropriate model export data based on format
        let model_data = match info.format {
            ModelFormat::Blend => api_response.blend.as_ref(),
            ModelFormat::Gltf => api_response.gltf.as_ref(),
            ModelFormat::Fbx => api_response.fbx.as_ref(),
            ModelFormat::Usd => api_response.usd.as_ref(),
        };

        let model_data = match model_data {
            Some(data) => data,
            None => return Ok(Vec::new()),
        };

        let mut downloads = Vec::new();

        // Extract each requested component
        for component in &info.components {
            if let Some(download) = self.extract_model_component(
                model_data,
                api_response,
                resolution_str,
                format_str,
                *component,
            )? {
                downloads.push(download);
            }
        }

        Ok(downloads)
    }

    fn extract_model_component(
        &self,
        model_data: &ModelExportData,
        api_response: &ModelApiResponse,
        resolution: &str,
        _format_name: &str,
        component: ModelAssetComponent,
    ) -> Result<Option<AssetDownload>> {
        // Get the resolution data
        let resolution_data = match resolution {
            "1k" => model_data.one_k.as_ref(),
            "2k" => model_data.two_k.as_ref(),
            "4k" => model_data.four_k.as_ref(),
            "8k" => model_data.eight_k.as_ref(),
            "16k" => model_data.sixteen_k.as_ref(),
            _ => return Ok(None),
        };

        let resolution_data = match resolution_data {
            Some(data) => data,
            None => return Ok(None),
        };

        match component {
            ModelAssetComponent::Blend => self.extract_model_file(resolution_data, "blend"),
            ModelAssetComponent::Gltf => self.extract_model_file(resolution_data, "gltf"),
            ModelAssetComponent::Fbx => self.extract_model_file(resolution_data, "fbx"),
            ModelAssetComponent::Usd => self.extract_model_file(resolution_data, "usd"),
            ModelAssetComponent::Diffuse => {
                if let Some(diffuse_data) = &api_response.diffuse {
                    let format_str = match &self {
                        AssetInfo::Model(info) => info.image_format.as_str(),
                        _ => "jpg",
                    };
                    self.extract_from_texture_map(diffuse_data, resolution, format_str, "diff")
                } else {
                    Ok(None)
                }
            }
            ModelAssetComponent::Normal => {
                // Extract DirectX normal maps from top-level nor_dx data
                if let Some(nor_dx_data) = &api_response.nor_dx {
                    let format_str = match &self {
                        AssetInfo::Model(info) => info.image_format.as_str(),
                        _ => "jpg",
                    };
                    self.extract_from_texture_map(nor_dx_data, resolution, format_str, "nor_dx")
                } else {
                    Ok(None)
                }
            }
            ModelAssetComponent::Rough => {
                if let Some(rough_data) = &api_response.rough {
                    let format_str = match &self {
                        AssetInfo::Model(info) => info.image_format.as_str(),
                        _ => "jpg",
                    };
                    self.extract_from_texture_map(rough_data, resolution, format_str, "rough")
                } else {
                    Ok(None)
                }
            }
            ModelAssetComponent::AO => {
                if let Some(ao_data) = &api_response.ao {
                    let format_str = match &self {
                        AssetInfo::Model(info) => info.image_format.as_str(),
                        _ => "jpg",
                    };
                    self.extract_from_texture_map(ao_data, resolution, format_str, "ao")
                } else {
                    Ok(None)
                }
            }
            ModelAssetComponent::AORoughMetal => {
                if let Some(arm_data) = &api_response.arm {
                    let format_str = match &self {
                        AssetInfo::Model(info) => info.image_format.as_str(),
                        _ => "jpg",
                    };
                    self.extract_from_texture_map(arm_data, resolution, format_str, "arm")
                } else {
                    Ok(None)
                }
            }
            ModelAssetComponent::Displacement => {
                if let Some(disp_data) = &api_response.displacement {
                    // Displacement maps are typically PNG for lossless height data
                    self.extract_from_texture_map(disp_data, resolution, "png", "disp")
                } else {
                    Ok(None)
                }
            }
        }
    }

    fn extract_model_file(
        &self,
        resolution_data: &ModelFileData,
        format_name: &str,
    ) -> Result<Option<AssetDownload>> {
        let file_info = match format_name {
            "blend" => resolution_data.blend.as_ref(),
            "gltf" => resolution_data.gltf.as_ref(),
            "fbx" => resolution_data.fbx.as_ref(),
            "usd" => resolution_data.usd.as_ref(),
            _ => return Ok(None),
        };

        let file_info = match file_info {
            Some(info) => info,
            None => return Ok(None),
        };

        let default_filename = format!("model.{format_name}");
        let filename = file_info
            .url
            .rsplit('/')
            .next()
            .unwrap_or(&default_filename);

        Ok(Some(AssetDownload {
            url: file_info.url.clone(),
            file_path: filename.to_string(),
            size: Some(file_info.size),
            md5: file_info.md5.clone(),
        }))
    }
}

/// Default texture components to download if none specified.
fn default_texture_components() -> Vec<TextureAssetComponent> {
    vec![
        TextureAssetComponent::Diffuse,
        TextureAssetComponent::Normal,
        TextureAssetComponent::Rough,
    ]
}

/// Default model components to download if none specified.
fn default_model_components() -> Vec<ModelAssetComponent> {
    vec![ModelAssetComponent::Gltf]
}

/// Poly Haven API response structure.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TextureApiResponse {
    #[serde(rename = "Diffuse")]
    diffuse: Option<TextureMapData>,
    #[serde(rename = "Rough")]
    rough: Option<TextureMapData>,
    #[serde(rename = "AO")]
    ao: Option<TextureMapData>,
    #[serde(rename = "Displacement")]
    displacement: Option<TextureMapData>,
    #[serde(rename = "nor_gl")]
    nor_gl: Option<TextureMapData>,
    #[serde(rename = "nor_dx")]
    nor_dx: Option<TextureMapData>,
    #[serde(rename = "arm")]
    arm: Option<TextureMapData>,
    // Material files for textures
    blend: Option<MaterialData>,
    gltf: Option<MaterialData>,
    mtlx: Option<MaterialData>,
}

/// Represents API response fields for model assets only
#[derive(Debug, Deserialize)]
struct ModelApiResponse {
    #[serde(rename = "Diffuse")]
    diffuse: Option<TextureMapData>,
    #[serde(rename = "Rough")]
    rough: Option<TextureMapData>,
    #[serde(rename = "AO")]
    ao: Option<TextureMapData>,
    #[serde(rename = "Displacement")]
    displacement: Option<TextureMapData>,
    #[serde(rename = "nor_gl")]
    #[allow(dead_code)]
    nor_gl: Option<TextureMapData>,
    #[serde(rename = "nor_dx")]
    nor_dx: Option<TextureMapData>,
    #[serde(rename = "arm")]
    arm: Option<TextureMapData>,
    // Model exports
    blend: Option<ModelExportData>,
    gltf: Option<ModelExportData>,
    fbx: Option<ModelExportData>,
    usd: Option<ModelExportData>,
}

/// Texture map data for different resolutions.
#[derive(Debug, Deserialize)]
struct TextureMapData {
    #[serde(rename = "1k")]
    one_k: Option<FormatData>,
    #[serde(rename = "2k")]
    two_k: Option<FormatData>,
    #[serde(rename = "4k")]
    four_k: Option<FormatData>,
    #[serde(rename = "8k")]
    eight_k: Option<FormatData>,
}

/// Material data for different resolutions (texture assets only).
#[derive(Debug, Deserialize)]
struct MaterialData {
    #[serde(rename = "1k")]
    one_k: Option<MaterialFileData>,
    #[serde(rename = "2k")]
    two_k: Option<MaterialFileData>,
    #[serde(rename = "4k")]
    four_k: Option<MaterialFileData>,
    #[serde(rename = "8k")]
    eight_k: Option<MaterialFileData>,
    #[serde(rename = "16k")]
    sixteen_k: Option<MaterialFileData>,
}

/// Model export data for different resolutions (model assets only).
#[derive(Debug, Deserialize)]
struct ModelExportData {
    #[serde(rename = "1k")]
    one_k: Option<ModelFileData>,
    #[serde(rename = "2k")]
    two_k: Option<ModelFileData>,
    #[serde(rename = "4k")]
    four_k: Option<ModelFileData>,
    #[serde(rename = "8k")]
    eight_k: Option<ModelFileData>,
    #[serde(rename = "16k")]
    sixteen_k: Option<ModelFileData>,
}

/// Format data for different image formats.
#[derive(Debug, Deserialize)]
struct FormatData {
    jpg: Option<FileInfo>,
    png: Option<FileInfo>,
    exr: Option<FileInfo>,
}

/// Material file data for different material formats (texture assets only).
#[derive(Debug, Deserialize)]
struct MaterialFileData {
    blend: Option<FileInfo>,
    gltf: Option<FileInfo>,
    mtlx: Option<FileInfo>,
}

/// Model file data for different model export formats (model assets only).
#[derive(Debug, Deserialize)]
struct ModelFileData {
    blend: Option<FileInfo>,
    gltf: Option<FileInfo>,
    fbx: Option<FileInfo>,
    usd: Option<FileInfo>,
}

/// File information from Poly Haven API.
#[derive(Debug, Deserialize)]
struct FileInfo {
    url: String,
    size: u64,
    md5: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const BRICK_WALL_001_RESPONSE: &str = include_str!("testdata/polyhaven/brick_wall_001.json");
    const MOON_ROC_01_RESPONSE: &str = include_str!("testdata/polyhaven/moon_rock_01.json");

    #[test]
    fn test_extract_single_component_2k_jpg() {
        let api_response: TextureApiResponse = serde_json::from_str(BRICK_WALL_001_RESPONSE)
            .expect("Failed to deserialize test API response");

        let asset_info = AssetInfo::Texture(TextureAssetInfo {
            id: "brick_wall_001".to_string(),
            resolution: TextureResolution::TwoK,
            format: ImageFormat::Jpg,
            components: vec![TextureAssetComponent::Diffuse],
        });

        let download = asset_info
            .extract_texture_download(
                &api_response,
                asset_info.as_texture_info(),
                TextureAssetComponent::Diffuse,
            )
            .expect("Failed to extract download")
            .expect("No download returned");

        assert_eq!(
            download.url,
            "https://dl.polyhaven.org/file/ph-assets/Textures/jpg/2k/brick_wall_001/brick_wall_001_diffuse_2k.jpg"
        );
        assert_eq!(download.file_path, "brick_wall_001_diffuse_2k.jpg");
        assert_eq!(download.size, Some(2043756));
        assert_eq!(
            download.md5,
            Some("0f478f67d252a1fa7b351460a19965bd".to_string())
        );
    }

    #[test]
    fn test_extract_multiple_components_2k_jpg() {
        let api_response: TextureApiResponse = serde_json::from_str(BRICK_WALL_001_RESPONSE)
            .expect("Failed to deserialize test API response");

        let asset_info = AssetInfo::Texture(TextureAssetInfo {
            id: "brick_wall_001".to_string(),
            resolution: TextureResolution::TwoK,
            format: ImageFormat::Jpg,
            components: vec![
                TextureAssetComponent::Diffuse,
                TextureAssetComponent::Rough,
                TextureAssetComponent::Normal,
            ],
        });

        // Test diffuse component
        let diffuse_download = asset_info
            .extract_texture_download(
                &api_response,
                asset_info.as_texture_info(),
                TextureAssetComponent::Diffuse,
            )
            .expect("Failed to extract diffuse download")
            .expect("No diffuse download returned");

        assert!(
            diffuse_download
                .url
                .contains("brick_wall_001_diffuse_2k.jpg")
        );
        assert_eq!(diffuse_download.size, Some(2043756));

        // Test rough component
        let rough_download = asset_info
            .extract_texture_download(
                &api_response,
                asset_info.as_texture_info(),
                TextureAssetComponent::Rough,
            )
            .expect("Failed to extract rough download")
            .expect("No rough download returned");

        assert!(rough_download.url.contains("brick_wall_001_rough_2k.jpg"));
        assert_eq!(rough_download.size, Some(788257));

        // Test normal component - should return Some since test data has nor_dx
        let normal_result = asset_info
            .extract_texture_download(
                &api_response,
                asset_info.as_texture_info(),
                TextureAssetComponent::Normal,
            )
            .expect("Failed to extract normal download")
            .expect("Expected Some for DirectX normal maps");

        assert!(normal_result.url.contains("brick_wall_001_nor_dx_2k.jpg"));
        assert_eq!(normal_result.size, Some(1160886));
    }

    #[test]
    fn test_extract_different_formats() {
        let api_response: TextureApiResponse = serde_json::from_str(BRICK_WALL_001_RESPONSE)
            .expect("Failed to deserialize test API response");

        // Test PNG format
        let asset_info_png = AssetInfo::Texture(TextureAssetInfo {
            id: "brick_wall_001".to_string(),
            resolution: TextureResolution::TwoK,
            format: ImageFormat::Png,
            components: vec![TextureAssetComponent::Diffuse],
        });

        let png_download = asset_info_png
            .extract_texture_download(
                &api_response,
                asset_info_png.as_texture_info(),
                TextureAssetComponent::Diffuse,
            )
            .expect("Failed to extract PNG download")
            .expect("No PNG download returned");

        assert!(png_download.url.contains("brick_wall_001_diffuse_2k.png"));
        assert_eq!(png_download.size, Some(6676088));

        // Test EXR format
        let asset_info_exr = AssetInfo::Texture(TextureAssetInfo {
            id: "brick_wall_001".to_string(),
            resolution: TextureResolution::TwoK,
            format: ImageFormat::Exr,
            components: vec![TextureAssetComponent::Diffuse],
        });

        let exr_download = asset_info_exr
            .extract_texture_download(
                &api_response,
                asset_info_exr.as_texture_info(),
                TextureAssetComponent::Diffuse,
            )
            .expect("Failed to extract EXR download")
            .expect("No EXR download returned");

        assert!(exr_download.url.contains("brick_wall_001_diffuse_2k.exr"));
        assert_eq!(exr_download.size, Some(5607889));
    }

    #[test]
    fn test_extract_material_components() {
        let api_response: TextureApiResponse = serde_json::from_str(BRICK_WALL_001_RESPONSE)
            .expect("Failed to deserialize test API response");

        let asset_info = AssetInfo::Texture(TextureAssetInfo {
            id: "brick_wall_001".to_string(),
            resolution: TextureResolution::TwoK,
            format: ImageFormat::Jpg,
            components: vec![TextureAssetComponent::Blend],
        });

        let blend_download = asset_info
            .extract_texture_download(
                &api_response,
                asset_info.as_texture_info(),
                TextureAssetComponent::Blend,
            )
            .expect("Failed to extract blend download")
            .expect("No blend download returned");

        assert!(blend_download.url.contains("brick_wall_001_2k.blend"));
        assert_eq!(blend_download.size, Some(278057));
    }

    #[test]
    fn test_extract_unavailable_component() {
        // Create an empty API response to simulate missing components
        let empty_api_response = TextureApiResponse {
            diffuse: None,
            rough: None,
            ao: None,
            displacement: None,
            nor_gl: None,
            nor_dx: None,
            arm: None,
            blend: None,
            gltf: None,
            mtlx: None,
        };

        let asset_info = AssetInfo::Texture(TextureAssetInfo {
            id: "brick_wall_001".to_string(),
            resolution: TextureResolution::OneK,
            format: ImageFormat::Jpg,
            components: vec![TextureAssetComponent::Diffuse],
        });

        // Diffuse component is not available in the empty API response
        let result = asset_info
            .extract_texture_download(
                &empty_api_response,
                asset_info.as_texture_info(),
                TextureAssetComponent::Diffuse,
            )
            .expect("Failed to extract download");

        assert!(result.is_none(), "Expected None for unavailable component");
    }

    #[test]
    fn test_extract_available_resolution() {
        let api_response: TextureApiResponse = serde_json::from_str(BRICK_WALL_001_RESPONSE)
            .expect("Failed to deserialize test API response");

        let asset_info = AssetInfo::Texture(TextureAssetInfo {
            id: "brick_wall_001".to_string(),
            resolution: TextureResolution::OneK,
            format: ImageFormat::Jpg,
            components: vec![TextureAssetComponent::Diffuse],
        });

        // Diffuse is available in our test data at 1K resolution
        let result = asset_info
            .extract_texture_download(
                &api_response,
                asset_info.as_texture_info(),
                TextureAssetComponent::Diffuse,
            )
            .expect("Failed to extract download");

        assert!(result.is_some(), "Expected Some for available component");
        let download = result.unwrap();
        assert!(download.url.contains("brick_wall_001_diffuse_1k.jpg"));
        assert_eq!(download.size, Some(574381));
    }

    #[test]
    fn test_extract_model_gltf_4k() {
        let model_api_response: ModelApiResponse = serde_json::from_str(MOON_ROC_01_RESPONSE)
            .expect("Failed to deserialize test API response");

        let asset_info = AssetInfo::Model(ModelAssetInfo {
            id: "moon_rock_01".to_string(),
            resolution: TextureResolution::FourK,
            format: ModelFormat::Gltf,
            image_format: ImageFormat::Jpg,
            components: vec![ModelAssetComponent::Gltf],
        });

        let downloads = asset_info
            .get_model_downloads_with_response(&model_api_response)
            .expect("Failed to extract model downloads");

        assert_eq!(downloads.len(), 1);
        let download = &downloads[0];

        assert!(download.url.contains("moon_rock_01_4k.gltf"));
        assert_eq!(download.file_path, "moon_rock_01_4k.gltf");
        assert_eq!(download.size, Some(7088));
        assert_eq!(
            download.md5,
            Some("a473f2866464d0382d0005225657f0f2".to_string())
        );
    }

    #[test]
    fn test_extract_model_blend_2k() {
        let model_api_response: ModelApiResponse = serde_json::from_str(MOON_ROC_01_RESPONSE)
            .expect("Failed to deserialize test API response");

        let asset_info = AssetInfo::Model(ModelAssetInfo {
            id: "moon_rock_01".to_string(),
            resolution: TextureResolution::TwoK,
            format: ModelFormat::Blend,
            image_format: ImageFormat::Jpg,
            components: vec![ModelAssetComponent::Blend],
        });

        let downloads = asset_info
            .get_model_downloads_with_response(&model_api_response)
            .expect("Failed to extract model downloads");

        assert_eq!(downloads.len(), 1);
        let download = &downloads[0];

        assert!(download.url.contains("moon_rock_01_2k.blend"));
        assert_eq!(download.file_path, "moon_rock_01_2k.blend");
        assert_eq!(download.size, Some(665287));
        assert_eq!(
            download.md5,
            Some("892454b2dbcc7b02f1e7e9f4d00b19cb".to_string())
        );
    }

    #[test]
    fn test_extract_model_fbx_8k() {
        let model_api_response: ModelApiResponse = serde_json::from_str(MOON_ROC_01_RESPONSE)
            .expect("Failed to deserialize test API response");

        let asset_info = AssetInfo::Model(ModelAssetInfo {
            id: "moon_rock_01".to_string(),
            resolution: TextureResolution::EightK,
            format: ModelFormat::Fbx,
            image_format: ImageFormat::Jpg,
            components: vec![ModelAssetComponent::Fbx],
        });

        let downloads = asset_info
            .get_model_downloads_with_response(&model_api_response)
            .expect("Failed to extract model downloads");

        assert_eq!(downloads.len(), 1);
        let download = &downloads[0];

        assert!(download.url.contains("moon_rock_01_8k.fbx"));
        assert_eq!(download.file_path, "moon_rock_01_8k.fbx");
        assert_eq!(download.size, Some(594460));
        assert_eq!(
            download.md5,
            Some("838ce756ddc18acaeaa1578e62867a8c".to_string())
        );
    }

    #[test]
    fn test_extract_model_usd_16k() {
        let model_api_response: ModelApiResponse = serde_json::from_str(MOON_ROC_01_RESPONSE)
            .expect("Failed to deserialize test API response");

        let asset_info = AssetInfo::Model(ModelAssetInfo {
            id: "moon_rock_01".to_string(),
            resolution: TextureResolution::SixteenK,
            format: ModelFormat::Usd,
            image_format: ImageFormat::Jpg,
            components: vec![ModelAssetComponent::Usd],
        });

        let downloads = asset_info
            .get_model_downloads_with_response(&model_api_response)
            .expect("Failed to extract model downloads");

        assert_eq!(downloads.len(), 1);
        let download = &downloads[0];

        assert!(download.url.contains("moon_rock_01_16k.usdc"));
        assert_eq!(download.file_path, "moon_rock_01_16k.usdc");
        assert_eq!(download.size, Some(701278));
        assert_eq!(
            download.md5,
            Some("466c01527c531d3c1f3c5910240e0f7c".to_string())
        );
    }

    #[test]
    fn test_extract_model_unavailable_format() {
        // Create an empty model API response to simulate missing model formats
        let empty_model_response = ModelApiResponse {
            diffuse: None,
            rough: None,
            ao: None,
            displacement: None,
            nor_gl: None,
            nor_dx: None,
            arm: None,
            blend: None,
            gltf: None,
            fbx: None,
            usd: None,
        };

        let asset_info = AssetInfo::Model(ModelAssetInfo {
            id: "moon_rock_01".to_string(),
            resolution: TextureResolution::FourK,
            format: ModelFormat::Gltf,
            image_format: ImageFormat::Jpg,
            components: vec![ModelAssetComponent::Gltf],
        });

        let downloads = asset_info
            .get_model_downloads_with_response(&empty_model_response)
            .expect("Failed to extract model downloads");

        assert_eq!(
            downloads.len(),
            0,
            "Expected empty downloads for unavailable model format"
        );
    }

    #[test]
    fn test_extract_model_unavailable_resolution() {
        let model_api_response: ModelApiResponse = serde_json::from_str(MOON_ROC_01_RESPONSE)
            .expect("Failed to deserialize test API response");

        let asset_info = AssetInfo::Model(ModelAssetInfo {
            id: "moon_rock_01".to_string(),
            resolution: TextureResolution::OneK, // Test 1k resolution availability
            format: ModelFormat::Usd,
            image_format: ImageFormat::Jpg,
            components: vec![ModelAssetComponent::Usd],
        });

        let downloads = asset_info
            .get_model_downloads_with_response(&model_api_response)
            .expect("Failed to extract model downloads");

        // 1k resolution should be available for USD format based on test data
        assert_eq!(downloads.len(), 1);
        let download = &downloads[0];

        assert!(download.url.contains("moon_rock_01_1k.usdc"));
        assert_eq!(download.file_path, "moon_rock_01_1k.usdc");
        assert_eq!(download.size, Some(701277));
        assert_eq!(
            download.md5,
            Some("cb5a72970dbbf4727a3df59b7388eb6a".to_string())
        );
    }

    #[test]
    fn test_extract_model_with_texture_components() {
        let model_api_response: ModelApiResponse = serde_json::from_str(MOON_ROC_01_RESPONSE)
            .expect("Failed to deserialize test API response");

        let asset_info = AssetInfo::Model(ModelAssetInfo {
            id: "moon_rock_01".to_string(),
            resolution: TextureResolution::TwoK,
            format: ModelFormat::Blend,
            image_format: ImageFormat::Exr,
            components: vec![
                ModelAssetComponent::Blend,
                ModelAssetComponent::Diffuse,
                ModelAssetComponent::Normal,
                ModelAssetComponent::Rough,
                ModelAssetComponent::Displacement,
            ],
        });

        let downloads = asset_info
            .get_model_downloads_with_response(&model_api_response)
            .expect("Failed to extract model downloads");

        // Should have 5 downloads: main model + 4 texture components
        assert_eq!(downloads.len(), 5);

        // Check main model file
        let model_download = downloads
            .iter()
            .find(|d| d.file_path.ends_with(".blend"))
            .unwrap();
        assert!(model_download.url.contains("moon_rock_01_2k.blend"));

        // Check diffuse texture
        let diffuse_download = downloads
            .iter()
            .find(|d| d.file_path.contains("_diff_"))
            .unwrap();
        assert!(diffuse_download.url.contains("_diff_2k.exr"));

        // Check normal texture (should be DirectX format)
        let normal_download = downloads
            .iter()
            .find(|d| d.file_path.contains("_nor_dx_"))
            .unwrap();
        assert!(normal_download.url.contains("_nor_dx_2k.exr"));

        // Check rough texture
        let rough_download = downloads
            .iter()
            .find(|d| d.file_path.contains("_rough_"))
            .unwrap();
        assert!(rough_download.url.contains("_rough_2k.exr"));

        // Check displacement texture
        let displacement_download = downloads
            .iter()
            .find(|d| d.file_path.contains("_disp_"))
            .unwrap();
        assert!(displacement_download.url.contains("_disp_2k.png"));
    }

    #[test]
    fn test_extract_model_only_main_component() {
        let model_api_response: ModelApiResponse = serde_json::from_str(MOON_ROC_01_RESPONSE)
            .expect("Failed to deserialize test API response");

        let asset_info = AssetInfo::Model(ModelAssetInfo {
            id: "moon_rock_01".to_string(),
            resolution: TextureResolution::FourK,
            format: ModelFormat::Gltf,
            image_format: ImageFormat::Jpg,
            components: vec![ModelAssetComponent::Gltf], // Only main model file
        });

        let downloads = asset_info
            .get_model_downloads_with_response(&model_api_response)
            .expect("Failed to extract model downloads");

        // Should have only 1 download: main model file
        assert_eq!(downloads.len(), 1);
        let download = &downloads[0];

        assert!(download.url.contains("moon_rock_01_4k.gltf"));
        assert_eq!(download.file_path, "moon_rock_01_4k.gltf");
    }

    #[test]
    fn test_extract_model_different_image_formats() {
        let model_api_response: ModelApiResponse = serde_json::from_str(MOON_ROC_01_RESPONSE)
            .expect("Failed to deserialize test API response");

        // Test JPG format (default)
        let asset_info_jpg = AssetInfo::Model(ModelAssetInfo {
            id: "moon_rock_01".to_string(),
            resolution: TextureResolution::TwoK,
            format: ModelFormat::Blend,
            image_format: ImageFormat::Jpg,
            components: vec![
                ModelAssetComponent::Blend,
                ModelAssetComponent::Diffuse,
                ModelAssetComponent::Rough,
            ],
        });

        let downloads_jpg = asset_info_jpg
            .get_model_downloads_with_response(&model_api_response)
            .expect("Failed to extract JPG downloads");

        // Check JPG format textures
        let diffuse_jpg = downloads_jpg
            .iter()
            .find(|d| d.file_path.contains("_diff_"))
            .unwrap();
        assert!(diffuse_jpg.url.contains("_diff_2k.jpg"));

        let rough_jpg = downloads_jpg
            .iter()
            .find(|d| d.file_path.contains("_rough_"))
            .unwrap();
        assert!(rough_jpg.url.contains("_rough_2k.jpg"));

        // Test PNG format
        let asset_info_png = AssetInfo::Model(ModelAssetInfo {
            id: "moon_rock_01".to_string(),
            resolution: TextureResolution::TwoK,
            format: ModelFormat::Blend,
            image_format: ImageFormat::Png,
            components: vec![
                ModelAssetComponent::Blend,
                ModelAssetComponent::Diffuse,
                ModelAssetComponent::Rough,
            ],
        });

        let downloads_png = asset_info_png
            .get_model_downloads_with_response(&model_api_response)
            .expect("Failed to extract PNG downloads");

        // Check PNG format textures
        let diffuse_png = downloads_png
            .iter()
            .find(|d| d.file_path.contains("_diff_"))
            .unwrap();
        assert!(diffuse_png.url.contains("_diff_2k.png"));

        let rough_png = downloads_png
            .iter()
            .find(|d| d.file_path.contains("_rough_"))
            .unwrap();
        assert!(rough_png.url.contains("_rough_2k.png"));
    }

    #[test]
    fn test_model_asset_info_default_image_format() {
        let model_api_response: ModelApiResponse = serde_json::from_str(MOON_ROC_01_RESPONSE)
            .expect("Failed to deserialize test API response");

        // Test default image format behavior (should be JPG)
        let asset_info = AssetInfo::Model(ModelAssetInfo {
            id: "moon_rock_01".to_string(),
            resolution: TextureResolution::TwoK,
            format: ModelFormat::Blend,
            image_format: ImageFormat::default(), // Should be JPG
            components: vec![ModelAssetComponent::Blend, ModelAssetComponent::Diffuse],
        });

        let downloads = asset_info
            .get_model_downloads_with_response(&model_api_response)
            .expect("Failed to extract downloads with default format");

        // Check that default format is JPG
        let diffuse_download = downloads
            .iter()
            .find(|d| d.file_path.contains("_diff_"))
            .unwrap();
        assert!(diffuse_download.url.contains("_diff_2k.jpg"));

        // Test JSON deserialization with explicit image_format field
        let json_with_image_format = r#"{
            "id": "test_asset",
            "resolution": "TwoK",
            "format": "Blend",
            "image_format": "Png",
            "components": ["Blend", "Diffuse"]
        }"#;

        let deserialized: ModelAssetInfo = serde_json::from_str(json_with_image_format)
            .expect("Failed to deserialize ModelAssetInfo with image_format field");

        assert_eq!(deserialized.image_format, ImageFormat::Png);
        assert_eq!(deserialized.id, "test_asset");
        assert_eq!(deserialized.resolution, TextureResolution::TwoK);
        assert_eq!(deserialized.format, ModelFormat::Blend);
    }
}
