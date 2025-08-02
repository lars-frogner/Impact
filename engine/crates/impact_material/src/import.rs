//! Importing materials from declarations.

use crate::setup::{fixed::FixedMaterialProperties, physical::PhysicalMaterialProperties};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MaterialDeclaration {
    pub name: String,
    pub properties: MaterialProperties,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum MaterialProperties {
    Fixed(FixedMaterialProperties),
    Physical(PhysicalMaterialProperties),
}
