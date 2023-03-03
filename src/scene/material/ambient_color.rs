//! Material with a global ambient color.

use crate::{
    geometry::VertexAttributeSet,
    rendering::{
        create_uniform_buffer_bind_group_layout_entry, GlobalAmbientColorShaderInput,
        MaterialShaderInput, UniformBufferable,
    },
    scene::{FixedMaterialResources, MaterialID, MaterialLibrary, MaterialSpecification, RGBColor},
};
use bytemuck::{Pod, Zeroable};
use impact_utils::{hash64, ConstStringHash64};
use lazy_static::lazy_static;

/// Material with a fixed ambient color that is the same for all uses of the
/// material.
///
/// This object is intended to be stored in a uniform buffer, so it is padded to
/// 16 bytes.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct GlobalAmbientColorMaterial {
    color: RGBColor,
    _padding: f32,
}

lazy_static! {
    static ref GLOBAL_AMBIENT_COLOR_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("GlobalAmbientColorMaterial"));
}

impl GlobalAmbientColorMaterial {
    pub const VERTEX_ATTRIBUTE_REQUIREMENTS: VertexAttributeSet = VertexAttributeSet::empty();

    /// Adds the material specification for this material to the given material
    /// library. The given ambient color is included in the specification as a
    /// fixed material resource.
    pub fn register(material_library: &mut MaterialLibrary, ambient_color: RGBColor) {
        let fixed_resources = FixedMaterialResources::new(&Self {
            color: ambient_color,
            _padding: 0.0,
        });

        let specification = MaterialSpecification::new(
            Self::VERTEX_ATTRIBUTE_REQUIREMENTS,
            Some(fixed_resources),
            Vec::new(),
            MaterialShaderInput::GlobalAmbientColor(GlobalAmbientColorShaderInput {
                uniform_binding: FixedMaterialResources::UNIFORM_BINDING,
            }),
        );

        material_library.add_material_specification(Self::material_id(), specification);
    }

    /// Returns the identifier of the material.
    pub fn material_id() -> MaterialID {
        *GLOBAL_AMBIENT_COLOR_MATERIAL_ID
    }
}

impl UniformBufferable for GlobalAmbientColorMaterial {
    const ID: ConstStringHash64 = ConstStringHash64::new("GlobalAmbientColorMaterial");

    fn create_bind_group_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        create_uniform_buffer_bind_group_layout_entry(binding, wgpu::ShaderStages::FRAGMENT)
    }
}
