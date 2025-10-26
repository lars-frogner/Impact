use crate::editor::meta::MetaNodeLink;

use super::{
    MetaFloatParam, MetaNodeID, MetaNodeParam, MetaNodeParams, MetaPortConfig, MetaUIntParam,
};
use impact::impact_containers::HashMap;
use impact_dev_ui::option_panels::LabelAndHoverText;
use impact_voxel::generation::sdf::meta::{
    MetaBoxSDF, MetaGradientNoiseSDF, MetaMultifractalNoiseSDFModifier,
    MetaMultiscaleSphereSDFModifier, MetaRotationToGradient, MetaSDFGroupUnion,
    MetaSDFIntersection, MetaSDFNode, MetaSDFNodeID, MetaSDFRotation, MetaSDFScaling,
    MetaSDFScattering, MetaSDFSubtraction, MetaSDFTranslation, MetaSDFUnion, MetaSphereSDF,
    MetaStochasticSelection, MetaStratifiedPlacement, MetaTranslationToSurface,
};

trait SpecificMetaNodeKind {
    const LABEL: &'static str;
    const PORT_CONFIG: MetaPortConfig;

    fn params() -> MetaNodeParams;

    fn build(
        id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetaNodeKind {
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
    GroupUnion,
    StratifiedPlacement,
    TranslationToSurface,
    RotationToGradient,
    Scattering,
    StochasticSelection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetaNodeKindGroup {
    Root,
    Primitive,
    Transform,
    Modification,
    Combination,
    Placement,
    Masking,
}

pub const DEFAULT_VOXEL_EXTENT: f32 = 0.25;
pub const MIN_VOXEL_EXTENT: f32 = 0.005;

impl SpecificMetaNodeKind for MetaBoxSDF {
    const LABEL: &'static str = "Box";
    const PORT_CONFIG: MetaPortConfig = MetaPortConfig::leaf();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Extent x"), 62.0)
                .with_min_value(0.0)
                .into(),
        );
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Extent y"), 62.0)
                .with_min_value(0.0)
                .into(),
        );
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Extent z"), 62.0)
                .with_min_value(0.0)
                .into(),
        );
        params
    }

    fn build(
        _id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
        _children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 3);
        let extents = [params[0].float(), params[1].float(), params[2].float()];
        Some(MetaSDFNode::new_box(extents.map(Into::into), 0))
    }
}

impl SpecificMetaNodeKind for MetaSphereSDF {
    const LABEL: &'static str = "Sphere";
    const PORT_CONFIG: MetaPortConfig = MetaPortConfig::leaf();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Radius"), 31.0)
                .with_min_value(0.0)
                .into(),
        );
        params
    }

    fn build(
        _id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
        _children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 1);
        let radius = params[0].float();
        Some(MetaSDFNode::new_sphere(radius.into(), 0))
    }
}

impl SpecificMetaNodeKind for MetaGradientNoiseSDF {
    const LABEL: &'static str = "Gradient noise";
    const PORT_CONFIG: MetaPortConfig = MetaPortConfig::leaf();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Extent x"), 62.0)
                .with_min_value(0.0)
                .into(),
        );
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Extent y"), 62.0)
                .with_min_value(0.0)
                .into(),
        );
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Extent z"), 62.0)
                .with_min_value(0.0)
                .into(),
        );
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Frequency"), 0.05)
                .with_min_value(0.0)
                .with_max_value(1.0)
                .with_speed(0.0002)
                .into(),
        );
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Threshold"), 0.0)
                .with_min_value(-1.0)
                .with_max_value(1.0)
                .with_speed(0.001)
                .into(),
        );
        params.push(MetaUIntParam::new(LabelAndHoverText::label_only("Seed"), 0).into());
        params
    }

    fn build(
        _id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
        _children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 6);
        let extents = [params[0].float(), params[1].float(), params[2].float()];
        let noise_frequency = params[3].float();
        let noise_threshold = params[4].float();
        let seed = params[5].uint();
        Some(MetaSDFNode::new_gradient_noise(
            extents.map(Into::into),
            noise_frequency.into(),
            noise_threshold.into(),
            seed,
        ))
    }
}

impl SpecificMetaNodeKind for MetaSDFTranslation {
    const LABEL: &'static str = "Translation";
    const PORT_CONFIG: MetaPortConfig = MetaPortConfig::unary();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("In x"), 0.0)
                .with_speed(0.05)
                .into(),
        );
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("In y"), 0.0)
                .with_speed(0.05)
                .into(),
        );
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("In z"), 0.0)
                .with_speed(0.05)
                .into(),
        );
        params
    }

    fn build(
        id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 3);
        let child_id = unary_child(id_map, children)?;
        let translation = [params[0].float(), params[1].float(), params[2].float()];
        Some(MetaSDFNode::new_translation(
            child_id,
            translation.map(Into::into),
            0,
        ))
    }
}

impl SpecificMetaNodeKind for MetaSDFRotation {
    const LABEL: &'static str = "Rotation";
    const PORT_CONFIG: MetaPortConfig = MetaPortConfig::unary();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Roll"), 0.0)
                .with_speed(0.002)
                .into(),
        );
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Pitch"), 0.0)
                .with_speed(0.002)
                .into(),
        );
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Yaw"), 0.0)
                .with_speed(0.002)
                .into(),
        );
        params
    }

    fn build(
        id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 3);
        let child_id = unary_child(id_map, children)?;
        let roll = params[0].float();
        let pitch = params[1].float();
        let yaw = params[2].float();
        Some(MetaSDFNode::new_rotation(
            child_id,
            roll.into(),
            pitch.into(),
            yaw.into(),
            0,
        ))
    }
}

impl SpecificMetaNodeKind for MetaSDFScaling {
    const LABEL: &'static str = "Scaling";
    const PORT_CONFIG: MetaPortConfig = MetaPortConfig::unary();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Factor"), 1.0)
                .with_min_value(1e-3)
                .with_speed(0.005)
                .into(),
        );
        params
    }

    fn build(
        id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 1);
        let child_id = unary_child(id_map, children)?;
        let scaling = params[0].float();
        Some(MetaSDFNode::new_scaling(child_id, scaling.into(), 0))
    }
}

impl SpecificMetaNodeKind for MetaMultifractalNoiseSDFModifier {
    const LABEL: &'static str = "Multifractal noise";
    const PORT_CONFIG: MetaPortConfig = MetaPortConfig::unary();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(MetaUIntParam::new(LabelAndHoverText::label_only("Octaves"), 1).into());
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Frequency"), 0.02)
                .with_min_value(0.0)
                .with_max_value(1.0)
                .with_speed(0.0002)
                .into(),
        );
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Lacunarity"), 2.0)
                .with_min_value(1.0)
                .with_max_value(10.0)
                .with_speed(0.001)
                .into(),
        );
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Persistence"), 0.5)
                .with_min_value(0.0)
                .with_max_value(1.0)
                .with_speed(0.001)
                .into(),
        );
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Amplitude"), 5.0)
                .with_min_value(0.0)
                .with_speed(0.05)
                .into(),
        );
        params.push(MetaUIntParam::new(LabelAndHoverText::label_only("Seed"), 0).into());
        params
    }

    fn build(
        id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 6);
        let child_id = unary_child(id_map, children)?;
        let octaves = params[0].uint();
        let frequency = params[1].float();
        let lacunarity = params[2].float();
        let persistence = params[3].float();
        let amplitude = params[4].float();
        let seed = params[5].uint();
        Some(MetaSDFNode::new_multifractal_noise(
            child_id,
            octaves.into(),
            frequency.into(),
            lacunarity.into(),
            persistence.into(),
            amplitude.into(),
            seed,
        ))
    }
}

impl SpecificMetaNodeKind for MetaMultiscaleSphereSDFModifier {
    const LABEL: &'static str = "Multiscale sphere";
    const PORT_CONFIG: MetaPortConfig = MetaPortConfig::unary();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(MetaUIntParam::new(LabelAndHoverText::label_only("Octaves"), 0).into());
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Max scale"), 10.0)
                .with_min_value(0.0)
                .with_speed(0.01)
                .into(),
        );
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Persistence"), 0.5)
                .with_min_value(0.0)
                .with_max_value(1.0)
                .with_speed(0.001)
                .into(),
        );
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Inflation"), 1.0)
                .with_min_value(0.0)
                .with_speed(0.005)
                .into(),
        );
        params.push(
            MetaFloatParam::new(
                LabelAndHoverText::label_only("Intersection smoothness"),
                1.0,
            )
            .with_min_value(0.0)
            .with_speed(0.001)
            .into(),
        );
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Union smoothness"), 0.3)
                .with_min_value(0.0)
                .with_speed(0.001)
                .into(),
        );
        params.push(MetaUIntParam::new(LabelAndHoverText::label_only("Seed"), 0).into());
        params
    }

    fn build(
        id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 7);
        let child_id = unary_child(id_map, children)?;
        let octaves = params[0].uint();
        let max_scale = params[1].float();
        let persistence = params[2].float();
        let inflation = params[3].float();
        let intersection_smoothness = params[4].float();
        let union_smoothness = params[5].float();
        let seed = params[6].uint();
        Some(MetaSDFNode::new_multiscale_sphere(
            child_id,
            octaves.into(),
            max_scale.into(),
            persistence.into(),
            inflation.into(),
            intersection_smoothness.into(),
            union_smoothness.into(),
            seed,
        ))
    }
}

impl SpecificMetaNodeKind for MetaSDFUnion {
    const LABEL: &'static str = "Union";
    const PORT_CONFIG: MetaPortConfig = MetaPortConfig::binary();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Smoothness"), 1.0)
                .with_min_value(0.0)
                .into(),
        );
        params
    }

    fn build(
        id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 1);
        let (child_1_id, child_2_id) = binary_children(id_map, children)?;
        let smoothness = params[0].float();
        Some(MetaSDFNode::new_union(child_1_id, child_2_id, smoothness))
    }
}

impl SpecificMetaNodeKind for MetaSDFSubtraction {
    const LABEL: &'static str = "Subtraction";
    const PORT_CONFIG: MetaPortConfig = MetaPortConfig::binary();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Smoothness"), 1.0)
                .with_min_value(0.0)
                .into(),
        );
        params
    }

    fn build(
        id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 1);
        let (child_1_id, child_2_id) = binary_children(id_map, children)?;
        let smoothness = params[0].float();
        Some(MetaSDFNode::new_subtraction(
            child_1_id, child_2_id, smoothness,
        ))
    }
}

impl SpecificMetaNodeKind for MetaSDFIntersection {
    const LABEL: &'static str = "Intersection";
    const PORT_CONFIG: MetaPortConfig = MetaPortConfig::binary();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Smoothness"), 1.0)
                .with_min_value(0.0)
                .into(),
        );
        params
    }

    fn build(
        id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 1);
        let (child_1_id, child_2_id) = binary_children(id_map, children)?;
        let smoothness = params[0].float();
        Some(MetaSDFNode::new_intersection(
            child_1_id, child_2_id, smoothness,
        ))
    }
}

impl SpecificMetaNodeKind for MetaSDFGroupUnion {
    const LABEL: &'static str = "Group union";
    const PORT_CONFIG: MetaPortConfig = MetaPortConfig::unary();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Smoothness"), 1.0)
                .with_min_value(0.0)
                .into(),
        );
        params
    }

    fn build(
        id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 1);
        let child_id = unary_child(id_map, children)?;
        let smoothness = params[0].float();
        Some(MetaSDFNode::new_group_union(child_id, smoothness))
    }
}

impl SpecificMetaNodeKind for MetaStratifiedPlacement {
    const LABEL: &'static str = "Stratified placement";
    const PORT_CONFIG: MetaPortConfig = MetaPortConfig::leaf();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(MetaUIntParam::new(LabelAndHoverText::label_only("Size x"), 1).into());
        params.push(MetaUIntParam::new(LabelAndHoverText::label_only("Size y"), 1).into());
        params.push(MetaUIntParam::new(LabelAndHoverText::label_only("Size z"), 1).into());
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Cell extent x"), 1.0)
                .with_min_value(0.0)
                .into(),
        );
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Cell extent y"), 1.0)
                .with_min_value(0.0)
                .into(),
        );
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Cell extent z"), 1.0)
                .with_min_value(0.0)
                .into(),
        );
        params.push(MetaUIntParam::new(LabelAndHoverText::label_only("Points per cell"), 1).into());
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Jitter fraction"), 0.0)
                .with_min_value(0.0)
                .with_max_value(1.0)
                .into(),
        );
        params.push(MetaUIntParam::new(LabelAndHoverText::label_only("Seed"), 0).into());
        params
    }

    fn build(
        _id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
        _children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 9);
        let shape = [params[0].uint(), params[1].uint(), params[2].uint()];
        let cell_extents = [params[3].float(), params[4].float(), params[5].float()];
        let points_per_grid_cell = params[6].uint();
        let jitter_fraction = params[7].float();
        let seed = params[8].uint();
        Some(MetaSDFNode::new_stratified_placement(
            shape.map(Into::into),
            cell_extents.map(Into::into),
            points_per_grid_cell.into(),
            jitter_fraction.into(),
            seed,
        ))
    }
}

impl SpecificMetaNodeKind for MetaTranslationToSurface {
    const LABEL: &'static str = "Translation to surface";
    const PORT_CONFIG: MetaPortConfig = MetaPortConfig::binary();

    fn params() -> MetaNodeParams {
        MetaNodeParams::new()
    }

    fn build(
        id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 0);
        let (surface_sdf_id, subject_id) = binary_children(id_map, children)?;
        Some(MetaSDFNode::new_translation_to_surface(
            surface_sdf_id,
            subject_id,
        ))
    }
}

impl SpecificMetaNodeKind for MetaRotationToGradient {
    const LABEL: &'static str = "Rotation to gradient";
    const PORT_CONFIG: MetaPortConfig = MetaPortConfig::binary();

    fn params() -> MetaNodeParams {
        MetaNodeParams::new()
    }

    fn build(
        id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 0);
        let (gradient_sdf_id, subject_id) = binary_children(id_map, children)?;
        Some(MetaSDFNode::new_rotation_to_gradient(
            gradient_sdf_id,
            subject_id,
        ))
    }
}

impl SpecificMetaNodeKind for MetaSDFScattering {
    const LABEL: &'static str = "Scattering";
    const PORT_CONFIG: MetaPortConfig = MetaPortConfig::binary();

    fn params() -> MetaNodeParams {
        MetaNodeParams::new()
    }

    fn build(
        id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 0);
        let (sdf_id, placement_id) = binary_children(id_map, children)?;
        Some(MetaSDFNode::new_scattering(sdf_id, placement_id))
    }
}

impl SpecificMetaNodeKind for MetaStochasticSelection {
    const LABEL: &'static str = "Stochastic selection";
    const PORT_CONFIG: MetaPortConfig = MetaPortConfig::unary();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Probability"), 1.0)
                .with_min_value(0.0)
                .with_max_value(1.0)
                .into(),
        );
        params.push(MetaUIntParam::new(LabelAndHoverText::label_only("Seed"), 0).into());
        params
    }

    fn build(
        id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 2);
        let child_id = unary_child(id_map, children)?;
        let probability = params[0].float();
        let seed = params[1].uint();
        Some(MetaSDFNode::new_stochastic_selection(
            child_id,
            probability,
            seed,
        ))
    }
}

impl MetaNodeKind {
    pub const fn all_non_root() -> [Self; 17] {
        [
            Self::Box,
            Self::Sphere,
            Self::GradientNoise,
            Self::Translation,
            Self::Rotation,
            Self::Scaling,
            Self::MultifractalNoise,
            Self::MultiscaleSphere,
            Self::Union,
            Self::Subtraction,
            Self::Intersection,
            Self::GroupUnion,
            Self::StratifiedPlacement,
            Self::TranslationToSurface,
            Self::RotationToGradient,
            Self::Scattering,
            Self::StochasticSelection,
        ]
    }

    pub fn is_root(&self) -> bool {
        *self == Self::Output
    }

    pub const fn group(&self) -> MetaNodeKindGroup {
        match self {
            Self::Output => MetaNodeKindGroup::Root,
            Self::Box | Self::Sphere | Self::GradientNoise => MetaNodeKindGroup::Primitive,
            Self::Translation | Self::Rotation | Self::Scaling => MetaNodeKindGroup::Transform,
            Self::MultifractalNoise | Self::MultiscaleSphere => MetaNodeKindGroup::Modification,
            Self::Union | Self::Subtraction | Self::Intersection | Self::GroupUnion => {
                MetaNodeKindGroup::Combination
            }
            Self::StratifiedPlacement
            | Self::TranslationToSurface
            | Self::RotationToGradient
            | Self::Scattering => MetaNodeKindGroup::Placement,
            Self::StochasticSelection => MetaNodeKindGroup::Masking,
        }
    }

    pub const fn label(&self) -> &'static str {
        match self {
            Self::Output => "Output",
            Self::Box => MetaBoxSDF::LABEL,
            Self::Sphere => MetaSphereSDF::LABEL,
            Self::GradientNoise => MetaGradientNoiseSDF::LABEL,
            Self::Translation => MetaSDFTranslation::LABEL,
            Self::Rotation => MetaSDFRotation::LABEL,
            Self::Scaling => MetaSDFScaling::LABEL,
            Self::MultifractalNoise => MetaMultifractalNoiseSDFModifier::LABEL,
            Self::MultiscaleSphere => MetaMultiscaleSphereSDFModifier::LABEL,
            Self::Union => MetaSDFUnion::LABEL,
            Self::Subtraction => MetaSDFSubtraction::LABEL,
            Self::Intersection => MetaSDFIntersection::LABEL,
            Self::GroupUnion => MetaSDFGroupUnion::LABEL,
            Self::StratifiedPlacement => MetaStratifiedPlacement::LABEL,
            Self::TranslationToSurface => MetaTranslationToSurface::LABEL,
            Self::RotationToGradient => MetaRotationToGradient::LABEL,
            Self::Scattering => MetaSDFScattering::LABEL,
            Self::StochasticSelection => MetaStochasticSelection::LABEL,
        }
    }

    pub const fn port_config(&self) -> MetaPortConfig {
        match self {
            Self::Output => MetaPortConfig::root(),
            Self::Box => MetaBoxSDF::PORT_CONFIG,
            Self::Sphere => MetaSphereSDF::PORT_CONFIG,
            Self::GradientNoise => MetaGradientNoiseSDF::PORT_CONFIG,
            Self::Translation => MetaSDFTranslation::PORT_CONFIG,
            Self::Rotation => MetaSDFRotation::PORT_CONFIG,
            Self::Scaling => MetaSDFScaling::PORT_CONFIG,
            Self::MultifractalNoise => MetaMultifractalNoiseSDFModifier::PORT_CONFIG,
            Self::MultiscaleSphere => MetaMultiscaleSphereSDFModifier::PORT_CONFIG,
            Self::Union => MetaSDFUnion::PORT_CONFIG,
            Self::Subtraction => MetaSDFSubtraction::PORT_CONFIG,
            Self::Intersection => MetaSDFIntersection::PORT_CONFIG,
            Self::GroupUnion => MetaSDFGroupUnion::PORT_CONFIG,
            Self::StratifiedPlacement => MetaStratifiedPlacement::PORT_CONFIG,
            Self::TranslationToSurface => MetaTranslationToSurface::PORT_CONFIG,
            Self::RotationToGradient => MetaRotationToGradient::PORT_CONFIG,
            Self::Scattering => MetaSDFScattering::PORT_CONFIG,
            Self::StochasticSelection => MetaStochasticSelection::PORT_CONFIG,
        }
    }

    pub fn params(&self) -> MetaNodeParams {
        match self {
            Self::Output => output_node_params(),
            Self::Box => MetaBoxSDF::params(),
            Self::Sphere => MetaSphereSDF::params(),
            Self::GradientNoise => MetaGradientNoiseSDF::params(),
            Self::Translation => MetaSDFTranslation::params(),
            Self::Rotation => MetaSDFRotation::params(),
            Self::Scaling => MetaSDFScaling::params(),
            Self::MultifractalNoise => MetaMultifractalNoiseSDFModifier::params(),
            Self::MultiscaleSphere => MetaMultiscaleSphereSDFModifier::params(),
            Self::Union => MetaSDFUnion::params(),
            Self::Subtraction => MetaSDFSubtraction::params(),
            Self::Intersection => MetaSDFIntersection::params(),
            Self::GroupUnion => MetaSDFGroupUnion::params(),
            Self::StratifiedPlacement => MetaStratifiedPlacement::params(),
            Self::TranslationToSurface => MetaTranslationToSurface::params(),
            Self::RotationToGradient => MetaRotationToGradient::params(),
            Self::Scattering => MetaSDFScattering::params(),
            Self::StochasticSelection => MetaStochasticSelection::params(),
        }
    }

    pub fn build_sdf_generator_node(
        &self,
        id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        match self {
            Self::Output => None,
            Self::Box => MetaBoxSDF::build(id_map, children, params),
            Self::Sphere => MetaSphereSDF::build(id_map, children, params),
            Self::GradientNoise => MetaGradientNoiseSDF::build(id_map, children, params),
            Self::Translation => MetaSDFTranslation::build(id_map, children, params),
            Self::Rotation => MetaSDFRotation::build(id_map, children, params),
            Self::Scaling => MetaSDFScaling::build(id_map, children, params),
            Self::MultifractalNoise => {
                MetaMultifractalNoiseSDFModifier::build(id_map, children, params)
            }
            Self::MultiscaleSphere => {
                MetaMultiscaleSphereSDFModifier::build(id_map, children, params)
            }
            Self::Union => MetaSDFUnion::build(id_map, children, params),
            Self::Subtraction => MetaSDFSubtraction::build(id_map, children, params),
            Self::Intersection => MetaSDFIntersection::build(id_map, children, params),
            Self::GroupUnion => MetaSDFGroupUnion::build(id_map, children, params),
            Self::StratifiedPlacement => MetaStratifiedPlacement::build(id_map, children, params),
            Self::TranslationToSurface => MetaTranslationToSurface::build(id_map, children, params),
            Self::RotationToGradient => MetaRotationToGradient::build(id_map, children, params),
            Self::Scattering => MetaSDFScattering::build(id_map, children, params),
            Self::StochasticSelection => MetaStochasticSelection::build(id_map, children, params),
        }
    }
}

impl MetaNodeKindGroup {
    pub const fn all_non_root() -> [Self; 6] {
        [
            Self::Primitive,
            Self::Transform,
            Self::Modification,
            Self::Combination,
            Self::Placement,
            Self::Masking,
        ]
    }
}

pub fn get_voxel_extent_from_output_node(output_node_params: &[MetaNodeParam]) -> f32 {
    output_node_params[0].float()
}

fn output_node_params() -> MetaNodeParams {
    let mut params = MetaNodeParams::new();
    params.push(
        MetaFloatParam::new(
            LabelAndHoverText::label_only("Voxel extent"),
            DEFAULT_VOXEL_EXTENT,
        )
        .with_min_value(MIN_VOXEL_EXTENT)
        .with_speed(0.01)
        .into(),
    );
    params
}

fn unary_child(
    id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
    children: &[Option<MetaNodeLink>],
) -> Option<MetaSDFNodeID> {
    assert_eq!(children.len(), 1);
    children
        .first()?
        .map(|attachment| id_map[&attachment.to_node])
}

fn binary_children(
    id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
    children: &[Option<MetaNodeLink>],
) -> Option<(MetaSDFNodeID, MetaSDFNodeID)> {
    assert_eq!(children.len(), 2);
    let child_0 = children
        .first()?
        .map(|attachment| id_map[&attachment.to_node])?;
    let child_1 = children
        .get(1)?
        .map(|attachment| id_map[&attachment.to_node])?;
    Some((child_0, child_1))
}
