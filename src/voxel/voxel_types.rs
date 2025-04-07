//! Voxel types and their properties.

use crate::io::util::parse_ron_file;
use anyhow::{Result, bail};
use bytemuck::{Pod, Zeroable};
use impact_utils::{Hash32, compute_hash_str_32};
use nalgebra::{Vector4, vector};
use nohash_hasher::BuildNoHashHasher;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::HashMap,
    path::{Path, PathBuf},
};

/// A type identifier that determines all the properties of a voxel.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Zeroable, Pod)]
pub struct VoxelType(u8);

#[derive(Clone, Debug, Default)]
pub struct VoxelTypeSpecifications(pub Vec<VoxelTypeSpecification>);

/// Specifies all relevant aspects of a voxel type.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoxelTypeSpecification {
    pub name: Cow<'static, str>,
    pub mass_density: f32,
    pub specular_reflectance: f32,
    pub roughness_scale: f32,
    pub metalness: f32,
    pub emissive_luminance: f32,
    pub color_texture_path: PathBuf,
    pub roughness_texture_path: PathBuf,
    pub normal_texture_path: PathBuf,
}

/// Registry containing the names and properties of all voxel types.
#[derive(Clone, Debug)]
pub struct VoxelTypeRegistry {
    name_lookup_table: HashMap<u32, VoxelType, BuildNoHashHasher<u32>>,
    names: Vec<Cow<'static, str>>,
    mass_densities: Vec<f32>,
    fixed_material_properties: Vec<FixedVoxelMaterialProperties>,
    color_texture_paths: Vec<PathBuf>,
    roughness_texture_paths: Vec<PathBuf>,
    normal_texture_paths: Vec<PathBuf>,
}

/// Specific properties of a voxel material that do not change with position.
#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub struct FixedVoxelMaterialProperties {
    properties: Vector4<f32>,
}

impl VoxelType {
    /// Creates a dummy voxel type that can not be present in any registry.
    pub const fn dummy() -> Self {
        Self(255)
    }

    /// Converts the given `u8` index into a [`VoxelType`].
    pub const fn from_idx_u8(idx: u8) -> Self {
        Self(idx)
    }

    /// Converts the given index into a [`VoxelType`].
    ///
    /// # Panics
    /// If `idx >= 256`.
    pub fn from_idx(idx: usize) -> Self {
        Self::from_idx_u8(u8::try_from(idx).unwrap())
    }

    /// Returns the index of the voxel type as a `u8`.
    pub const fn idx_u8(&self) -> u8 {
        self.0
    }

    /// Returns the index of the voxel type.
    pub const fn idx(&self) -> usize {
        self.0 as usize
    }
}

impl VoxelTypeRegistry {
    /// The maximum number of voxel types that can be registered.
    pub const fn max_n_voxel_types() -> usize {
        255
    }

    /// Reads the RON (Rusty Object Notation) file at the given path and
    /// deserializes it into a [`VoxelTypeSpecifications`] object that is used
    /// to create a new voxel type registry.
    pub fn from_voxel_type_ron_file(file_path: impl AsRef<Path>) -> Result<Self> {
        let voxel_types = parse_ron_file(file_path)?;
        Self::new(VoxelTypeSpecifications(voxel_types))
    }

    /// Creates a new voxel type registry for the specified voxel types.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The number of voxel types is not smaller than
    ///   [`Self::max_voxel_types`].
    /// - There are duplicate names.
    pub fn new(voxel_types: VoxelTypeSpecifications) -> Result<Self> {
        voxel_types.validate()?;

        let (
            names,
            mass_densities,
            fixed_material_properties,
            color_texture_paths,
            roughness_texture_paths,
            normal_texture_paths,
        ) = voxel_types.unzip();

        let name_lookup_table: HashMap<_, _, _> = names
            .iter()
            .enumerate()
            .map(|(idx, name)| (compute_hash_str_32(name).into(), VoxelType::from_idx(idx)))
            .collect();

        if name_lookup_table.len() != names.len() {
            bail!("Duplicate voxel type names in registry");
        }

        Ok(Self {
            name_lookup_table,
            names,
            mass_densities,
            fixed_material_properties,
            color_texture_paths,
            roughness_texture_paths,
            normal_texture_paths,
        })
    }

    /// Returns the number of registered voxel types.
    pub fn n_voxel_types(&self) -> usize {
        self.names.len()
    }

    /// Returns the voxel type with the given name, or [`None`] if no voxel type
    /// with the given name has been registered.
    pub fn voxel_type_for_name(&self, name: &str) -> Option<VoxelType> {
        self.voxel_type_for_name_hash(compute_hash_str_32(name))
    }

    /// Returns the voxel type with the given name hash, or [`None`] if no voxel
    /// type with the given name has been registered.
    pub fn voxel_type_for_name_hash(&self, name_hash: Hash32) -> Option<VoxelType> {
        self.name_lookup_table.get(&name_hash.into()).copied()
    }

    /// Returns the name of the given voxel type.
    ///
    /// # Panics
    /// If the voxel type is not present in the registry.
    pub fn name(&self, voxel_type: VoxelType) -> &str {
        self.get_name(voxel_type)
            .expect("Voxel type not present in registry")
    }

    /// Returns the name of the given voxel type, or [`None`] if the voxel type
    /// is not present in the registry.
    pub fn get_name(&self, voxel_type: VoxelType) -> Option<&str> {
        self.names.get(voxel_type.idx()).map(|name| name.as_ref())
    }

    /// Returns the slice of mass densities for all registered voxel types.
    pub fn mass_densities(&self) -> &[f32] {
        &self.mass_densities
    }

    /// Returns the slice of fixed material properties for all registered voxel
    /// types.
    pub fn fixed_material_properties(&self) -> &[FixedVoxelMaterialProperties] {
        &self.fixed_material_properties
    }

    /// Returns the slice of color texture paths for all registered voxel types.
    pub fn color_texture_paths(&self) -> &[PathBuf] {
        &self.color_texture_paths
    }

    /// Returns the slice of roughness texture paths for all registered voxel
    /// types.
    pub fn roughness_texture_paths(&self) -> &[PathBuf] {
        &self.roughness_texture_paths
    }

    /// Returns the slice of normal texture paths for all registered voxel
    /// types.
    pub fn normal_texture_paths(&self) -> &[PathBuf] {
        &self.normal_texture_paths
    }
}

impl VoxelTypeSpecifications {
    fn validate(&self) -> Result<()> {
        if self.0.len() >= VoxelTypeRegistry::max_n_voxel_types() {
            bail!("Too many voxel types for registry");
        }
        Ok(())
    }

    #[allow(clippy::type_complexity)]
    fn unzip(
        self,
    ) -> (
        Vec<Cow<'static, str>>,
        Vec<f32>,
        Vec<FixedVoxelMaterialProperties>,
        Vec<PathBuf>,
        Vec<PathBuf>,
        Vec<PathBuf>,
    ) {
        let mut names = Vec::with_capacity(self.0.len());
        let mut mass_densities = Vec::with_capacity(self.0.len());
        let mut fixed_material_properties = Vec::with_capacity(self.0.len());
        let mut color_texture_paths = Vec::with_capacity(self.0.len());
        let mut roughness_texture_paths = Vec::with_capacity(self.0.len());
        let mut normal_texture_paths = Vec::with_capacity(self.0.len());

        for VoxelTypeSpecification {
            name,
            mass_density,
            specular_reflectance,
            roughness_scale,
            metalness,
            emissive_luminance,
            color_texture_path,
            roughness_texture_path,
            normal_texture_path,
        } in self.0
        {
            names.push(name);
            mass_densities.push(mass_density);
            fixed_material_properties.push(FixedVoxelMaterialProperties::new(
                specular_reflectance,
                roughness_scale,
                metalness,
                emissive_luminance,
            ));
            color_texture_paths.push(color_texture_path);
            roughness_texture_paths.push(roughness_texture_path);
            normal_texture_paths.push(normal_texture_path);
        }

        (
            names,
            mass_densities,
            fixed_material_properties,
            color_texture_paths,
            roughness_texture_paths,
            normal_texture_paths,
        )
    }
}

impl FixedVoxelMaterialProperties {
    /// Combines the given fixed properties for a voxel material.
    pub fn new(
        specular_reflectance: f32,
        roughness_scale: f32,
        metalness: f32,
        emissive_luminance: f32,
    ) -> Self {
        Self {
            properties: vector![
                specular_reflectance,
                roughness_scale,
                metalness,
                emissive_luminance
            ],
        }
    }
}

impl Default for FixedVoxelMaterialProperties {
    fn default() -> Self {
        Self::new(0.5, 0.5, 0.0, 0.0)
    }
}
