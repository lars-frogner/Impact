use super::{AtomicFloatParam, AtomicNode, AtomicNodeParams, AtomicPortConfig, AtomicUIntParam};
use impact_voxel::generation::sdf::{
    BoxSDF, GradientNoiseSDF, MultifractalNoiseSDFModifier, MultiscaleSphereSDFModifier,
    SDFIntersection, SDFRotation, SDFScaling, SDFSubtraction, SDFTranslation, SDFUnion, SphereSDF,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtomicNodeKind {
    Output,
    Box,
    Sphere,
    GradientNoise,
    Translation,
    Rotation,
    Scaling,
    MultifractalNoise,
    MultiscaleSphere,
    Union,
    Subtraction,
    Intersection,
}

impl AtomicNode {
    pub fn for_box(node: &BoxSDF) -> Self {
        let extents = node.extents();
        let mut params = AtomicNodeParams::new();
        params.push(AtomicFloatParam::new("Extent x", extents[0]).into());
        params.push(AtomicFloatParam::new("Extent y", extents[1]).into());
        params.push(AtomicFloatParam::new("Extent z", extents[2]).into());
        Self::new_leaf(AtomicNodeKind::Box, params)
    }

    pub fn for_sphere(node: &SphereSDF) -> Self {
        let mut params = AtomicNodeParams::new();
        params.push(AtomicFloatParam::new("Radius", node.radius()).into());
        Self::new_leaf(AtomicNodeKind::Sphere, params)
    }

    pub fn for_gradient_noise(node: &GradientNoiseSDF) -> Self {
        let extents = node.extents();
        let mut params = AtomicNodeParams::new();
        params.push(AtomicFloatParam::new("Extent x", extents[0]).into());
        params.push(AtomicFloatParam::new("Extent y", extents[1]).into());
        params.push(AtomicFloatParam::new("Extent z", extents[2]).into());
        params.push(AtomicFloatParam::new("Frequency", node.noise_frequency()).into());
        params.push(AtomicFloatParam::new("Threshold", node.noise_threshold()).into());
        params.push(AtomicUIntParam::new("Seed", node.seed()).into());
        Self::new_leaf(AtomicNodeKind::GradientNoise, params)
    }

    pub fn for_translation(node: &SDFTranslation) -> Self {
        let mut params = AtomicNodeParams::new();
        params.push(AtomicFloatParam::new("X", node.translation.x).into());
        params.push(AtomicFloatParam::new("Y", node.translation.y).into());
        params.push(AtomicFloatParam::new("Z", node.translation.z).into());
        Self::new_unary(AtomicNodeKind::Translation, params, node.child_id)
    }

    pub fn for_rotation(node: &SDFRotation) -> Self {
        let (roll, pitch, yaw) = node.rotation.euler_angles();
        let mut params = AtomicNodeParams::new();
        params.push(AtomicFloatParam::new("Roll", roll).into());
        params.push(AtomicFloatParam::new("Pitch", pitch).into());
        params.push(AtomicFloatParam::new("Yaw", yaw).into());
        Self::new_unary(AtomicNodeKind::Rotation, params, node.child_id)
    }

    pub fn for_scaling(node: &SDFScaling) -> Self {
        let mut params = AtomicNodeParams::new();
        params.push(AtomicFloatParam::new("Scaling", node.scaling).into());
        Self::new_unary(AtomicNodeKind::Scaling, params, node.child_id)
    }

    pub fn for_multifractal_noise(node: &MultifractalNoiseSDFModifier) -> Self {
        let mut params = AtomicNodeParams::new();
        params.push(AtomicUIntParam::new("Octaves", node.octaves()).into());
        params.push(AtomicFloatParam::new("Frequency", node.frequency()).into());
        params.push(AtomicFloatParam::new("Lacunarity", node.lacunarity()).into());
        params.push(AtomicFloatParam::new("Persistence", node.persistence()).into());
        params.push(AtomicFloatParam::new("Amplitude", node.amplitude()).into());
        params.push(AtomicUIntParam::new("Seed", node.seed()).into());
        Self::new_unary(AtomicNodeKind::MultifractalNoise, params, node.child_id)
    }

    pub fn for_multiscale_sphere(node: &MultiscaleSphereSDFModifier) -> Self {
        let mut params = AtomicNodeParams::new();
        params.push(AtomicUIntParam::new("Octaves", node.octaves()).into());
        params.push(AtomicFloatParam::new("Max scale", node.max_scale()).into());
        params.push(AtomicFloatParam::new("Persistence", node.persistence()).into());
        params.push(AtomicFloatParam::new("Inflation", node.inflation()).into());
        params.push(
            AtomicFloatParam::new("Intersection smoothness", node.intersection_smoothness()).into(),
        );
        params.push(AtomicFloatParam::new("Union smoothness", node.union_smoothness()).into());
        params.push(AtomicUIntParam::new("Seed", node.seed()).into());
        Self::new_unary(AtomicNodeKind::MultiscaleSphere, params, node.child_id)
    }

    pub fn for_union(node: &SDFUnion) -> Self {
        let mut params = AtomicNodeParams::new();
        params.push(AtomicFloatParam::new("Smoothness", node.smoothness).into());
        Self::new_binary(
            AtomicNodeKind::Union,
            params,
            node.child_1_id,
            node.child_2_id,
        )
    }

    pub fn for_subtraction(node: &SDFSubtraction) -> Self {
        let mut params = AtomicNodeParams::new();
        params.push(AtomicFloatParam::new("Smoothness", node.smoothness).into());
        Self::new_binary(
            AtomicNodeKind::Subtraction,
            params,
            node.child_1_id,
            node.child_2_id,
        )
    }

    pub fn for_intersection(node: &SDFIntersection) -> Self {
        let mut params = AtomicNodeParams::new();
        params.push(AtomicFloatParam::new("Smoothness", node.smoothness).into());
        Self::new_binary(
            AtomicNodeKind::Intersection,
            params,
            node.child_1_id,
            node.child_2_id,
        )
    }
}

impl AtomicNodeKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Output => "Output",
            Self::Box => "Box",
            Self::Sphere => "Sphere",
            Self::GradientNoise => "Gradient noise",
            Self::Translation => "Translation",
            Self::Rotation => "Rotation",
            Self::Scaling => "Scaling",
            Self::MultifractalNoise => "Multifractal noise",
            Self::MultiscaleSphere => "Multiscale sphere",
            Self::Union => "Union",
            Self::Subtraction => "Subtraction",
            Self::Intersection => "Intersection",
        }
    }

    pub fn port_config(&self) -> AtomicPortConfig {
        match self {
            Self::Output => AtomicPortConfig::root(),
            Self::Box | Self::Sphere | Self::GradientNoise => AtomicPortConfig::leaf(),
            Self::Translation
            | Self::Rotation
            | Self::Scaling
            | Self::MultifractalNoise
            | Self::MultiscaleSphere => AtomicPortConfig::unary(),
            Self::Union | Self::Subtraction | Self::Intersection => AtomicPortConfig::binary(),
        }
    }
}
