use super::{
    MetaFloatParam, MetaFloatRangeParam, MetaNodeID, MetaNodeLink, MetaNodeParam, MetaNodeParams,
    MetaUIntParam, MetaUIntRangeParam,
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
use serde::{Deserialize, Serialize};

trait SpecificMetaNodeKind {
    const LABEL: &'static str;
    const PARENT_PORT_KIND: MetaParentPortKind;
    const CHILD_PORT_KINDS: MetaChildPortKinds;

    fn params() -> MetaNodeParams;

    fn build(
        id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MetaChildPortKind {
    #[default]
    SingleSDF,
    SDFGroup,
    SinglePlacement,
    PlacementGroup,
    Any,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MetaParentPortKind {
    #[default]
    SingleSDF,
    SDFGroup,
    SinglePlacement,
    PlacementGroup,
    SameAsInput {
        slot: usize,
    },
}

const MAX_CHILD_PORTS: usize = 2;

type MetaChildPortKinds = [Option<MetaChildPortKind>; MAX_CHILD_PORTS];

pub const DEFAULT_VOXEL_EXTENT: f32 = 0.25;
pub const MIN_VOXEL_EXTENT: f32 = 0.005;

impl SpecificMetaNodeKind for MetaBoxSDF {
    const LABEL: &'static str = "Box";
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SingleSDF;
    const CHILD_PORT_KINDS: MetaChildPortKinds = leaf_child_port_kind();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatRangeParam::new_single_value(LabelAndHoverText::label_only("Extent x"), 62.0)
                .with_min_value(0.0)
                .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(LabelAndHoverText::label_only("Extent y"), 62.0)
                .with_min_value(0.0)
                .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(LabelAndHoverText::label_only("Extent z"), 62.0)
                .with_min_value(0.0)
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
        assert_eq!(params.len(), 4);
        let extents = [
            params[0].float_range(),
            params[1].float_range(),
            params[2].float_range(),
        ];
        let seed = params[3].uint();
        Some(MetaSDFNode::new_box(extents, seed))
    }
}

impl SpecificMetaNodeKind for MetaSphereSDF {
    const LABEL: &'static str = "Sphere";
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SingleSDF;
    const CHILD_PORT_KINDS: MetaChildPortKinds = leaf_child_port_kind();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatRangeParam::new_single_value(LabelAndHoverText::label_only("Radius"), 31.0)
                .with_min_value(0.0)
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
        assert_eq!(params.len(), 2);
        let radius = params[0].float_range();
        let seed = params[1].uint();
        Some(MetaSDFNode::new_sphere(radius, seed))
    }
}

impl SpecificMetaNodeKind for MetaGradientNoiseSDF {
    const LABEL: &'static str = "Gradient noise";
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SingleSDF;
    const CHILD_PORT_KINDS: MetaChildPortKinds = leaf_child_port_kind();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatRangeParam::new_single_value(LabelAndHoverText::label_only("Extent x"), 62.0)
                .with_min_value(0.0)
                .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(LabelAndHoverText::label_only("Extent y"), 62.0)
                .with_min_value(0.0)
                .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(LabelAndHoverText::label_only("Extent z"), 62.0)
                .with_min_value(0.0)
                .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(LabelAndHoverText::label_only("Frequency"), 0.05)
                .with_min_value(0.0)
                .with_max_value(1.0)
                .with_speed(0.0002)
                .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(LabelAndHoverText::label_only("Threshold"), 0.0)
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
        let extents = [
            params[0].float_range(),
            params[1].float_range(),
            params[2].float_range(),
        ];
        let noise_frequency = params[3].float_range();
        let noise_threshold = params[4].float_range();
        let seed = params[5].uint();
        Some(MetaSDFNode::new_gradient_noise(
            extents,
            noise_frequency,
            noise_threshold,
            seed,
        ))
    }
}

impl SpecificMetaNodeKind for MetaSDFTranslation {
    const LABEL: &'static str = "Translation";
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SameAsInput { slot: 0 };
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        single_child_port_kind(MetaChildPortKind::SDFGroup);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatRangeParam::new_single_value(LabelAndHoverText::label_only("In x"), 0.0)
                .with_speed(0.05)
                .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(LabelAndHoverText::label_only("In y"), 0.0)
                .with_speed(0.05)
                .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(LabelAndHoverText::label_only("In z"), 0.0)
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
        assert_eq!(params.len(), 4);
        let child_id = unary_child(id_map, children)?;
        let translation = [
            params[0].float_range(),
            params[1].float_range(),
            params[2].float_range(),
        ];
        let seed = params[3].uint();
        Some(MetaSDFNode::new_translation(child_id, translation, seed))
    }
}

impl SpecificMetaNodeKind for MetaSDFRotation {
    const LABEL: &'static str = "Rotation";
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SameAsInput { slot: 0 };
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        single_child_port_kind(MetaChildPortKind::SDFGroup);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatRangeParam::new_single_value(LabelAndHoverText::label_only("Roll"), 0.0)
                .with_speed(0.002)
                .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(LabelAndHoverText::label_only("Pitch"), 0.0)
                .with_speed(0.002)
                .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(LabelAndHoverText::label_only("Yaw"), 0.0)
                .with_speed(0.002)
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
        assert_eq!(params.len(), 4);
        let child_id = unary_child(id_map, children)?;
        let roll = params[0].float_range();
        let pitch = params[1].float_range();
        let yaw = params[2].float_range();
        let seed = params[3].uint();
        Some(MetaSDFNode::new_rotation(child_id, roll, pitch, yaw, seed))
    }
}

impl SpecificMetaNodeKind for MetaSDFScaling {
    const LABEL: &'static str = "Scaling";
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SameAsInput { slot: 0 };
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        single_child_port_kind(MetaChildPortKind::SDFGroup);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatRangeParam::new_single_value(LabelAndHoverText::label_only("Factor"), 1.0)
                .with_min_value(1e-3)
                .with_speed(0.005)
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
        let scaling = params[0].float_range();
        let seed = params[1].uint();
        Some(MetaSDFNode::new_scaling(child_id, scaling, seed))
    }
}

impl SpecificMetaNodeKind for MetaMultifractalNoiseSDFModifier {
    const LABEL: &'static str = "Multifractal noise";
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SameAsInput { slot: 0 };
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        single_child_port_kind(MetaChildPortKind::SDFGroup);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaUIntRangeParam::new_single_value(LabelAndHoverText::label_only("Octaves"), 1)
                .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(LabelAndHoverText::label_only("Frequency"), 0.02)
                .with_min_value(0.0)
                .with_max_value(1.0)
                .with_speed(0.0002)
                .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(LabelAndHoverText::label_only("Lacunarity"), 2.0)
                .with_min_value(1.0)
                .with_max_value(10.0)
                .with_speed(0.001)
                .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText::label_only("Persistence"),
                0.5,
            )
            .with_min_value(0.0)
            .with_max_value(1.0)
            .with_speed(0.001)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(LabelAndHoverText::label_only("Amplitude"), 5.0)
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
        let octaves = params[0].uint_range();
        let frequency = params[1].float_range();
        let lacunarity = params[2].float_range();
        let persistence = params[3].float_range();
        let amplitude = params[4].float_range();
        let seed = params[5].uint();
        Some(MetaSDFNode::new_multifractal_noise(
            child_id,
            octaves,
            frequency,
            lacunarity,
            persistence,
            amplitude,
            seed,
        ))
    }
}

impl SpecificMetaNodeKind for MetaMultiscaleSphereSDFModifier {
    const LABEL: &'static str = "Multiscale sphere";
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SameAsInput { slot: 0 };
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        single_child_port_kind(MetaChildPortKind::SDFGroup);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaUIntRangeParam::new_single_value(LabelAndHoverText::label_only("Octaves"), 0)
                .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(LabelAndHoverText::label_only("Max scale"), 10.0)
                .with_min_value(0.0)
                .with_speed(0.01)
                .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText::label_only("Persistence"),
                0.5,
            )
            .with_min_value(0.0)
            .with_max_value(1.0)
            .with_speed(0.001)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(LabelAndHoverText::label_only("Inflation"), 1.0)
                .with_min_value(0.0)
                .with_speed(0.005)
                .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText::label_only("Intersection smoothness"),
                1.0,
            )
            .with_min_value(0.0)
            .with_speed(0.001)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText::label_only("Union smoothness"),
                0.3,
            )
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
        let octaves = params[0].uint_range();
        let max_scale = params[1].float_range();
        let persistence = params[2].float_range();
        let inflation = params[3].float_range();
        let intersection_smoothness = params[4].float_range();
        let union_smoothness = params[5].float_range();
        let seed = params[6].uint();
        Some(MetaSDFNode::new_multiscale_sphere(
            child_id,
            octaves,
            max_scale,
            persistence,
            inflation,
            intersection_smoothness,
            union_smoothness,
            seed,
        ))
    }
}

impl SpecificMetaNodeKind for MetaSDFUnion {
    const LABEL: &'static str = "Union";
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SingleSDF;
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        two_child_port_kinds(MetaChildPortKind::SingleSDF, MetaChildPortKind::SingleSDF);

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
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SingleSDF;
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        two_child_port_kinds(MetaChildPortKind::SingleSDF, MetaChildPortKind::SingleSDF);

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
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SingleSDF;
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        two_child_port_kinds(MetaChildPortKind::SingleSDF, MetaChildPortKind::SingleSDF);

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
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SingleSDF;
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        single_child_port_kind(MetaChildPortKind::SDFGroup);

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
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::PlacementGroup;
    const CHILD_PORT_KINDS: MetaChildPortKinds = leaf_child_port_kind();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(MetaUIntParam::new(LabelAndHoverText::label_only("Size x"), 1).into());
        params.push(MetaUIntParam::new(LabelAndHoverText::label_only("Size y"), 1).into());
        params.push(MetaUIntParam::new(LabelAndHoverText::label_only("Size z"), 1).into());
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Cell extent x"), 62.0)
                .with_min_value(0.0)
                .into(),
        );
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Cell extent y"), 62.0)
                .with_min_value(0.0)
                .into(),
        );
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Cell extent z"), 62.0)
                .with_min_value(0.0)
                .into(),
        );
        params.push(MetaUIntParam::new(LabelAndHoverText::label_only("Points per cell"), 1).into());
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Jitter fraction"), 0.0)
                .with_min_value(0.0)
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
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SameAsInput { slot: 1 };
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        two_child_port_kinds(MetaChildPortKind::SingleSDF, MetaChildPortKind::Any);

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
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SameAsInput { slot: 1 };
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        two_child_port_kinds(MetaChildPortKind::SingleSDF, MetaChildPortKind::Any);

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
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SDFGroup;
    const CHILD_PORT_KINDS: MetaChildPortKinds = two_child_port_kinds(
        MetaChildPortKind::SDFGroup,
        MetaChildPortKind::PlacementGroup,
    );

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
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SameAsInput { slot: 0 };
    const CHILD_PORT_KINDS: MetaChildPortKinds = single_child_port_kind(MetaChildPortKind::Any);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(MetaUIntRangeParam::new(LabelAndHoverText::label_only("Count"), 1, 1).into());
        params.push(
            MetaFloatParam::new(LabelAndHoverText::label_only("Probability"), 1.0)
                .with_min_value(0.0)
                .with_max_value(1.0)
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
        assert_eq!(params.len(), 3);
        let child_id = unary_child(id_map, children)?;
        let pick_count = params[0].uint_range();
        let pick_probability = params[1].float();
        let seed = params[2].uint();
        Some(MetaSDFNode::new_stochastic_selection(
            child_id,
            pick_count.min..=pick_count.max,
            pick_probability,
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

    pub fn is_output(&self) -> bool {
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

    pub const fn parent_port_kind(&self) -> MetaParentPortKind {
        match self {
            Self::Output => MetaParentPortKind::SingleSDF,
            Self::Box => MetaBoxSDF::PARENT_PORT_KIND,
            Self::Sphere => MetaSphereSDF::PARENT_PORT_KIND,
            Self::GradientNoise => MetaGradientNoiseSDF::PARENT_PORT_KIND,
            Self::Translation => MetaSDFTranslation::PARENT_PORT_KIND,
            Self::Rotation => MetaSDFRotation::PARENT_PORT_KIND,
            Self::Scaling => MetaSDFScaling::PARENT_PORT_KIND,
            Self::MultifractalNoise => MetaMultifractalNoiseSDFModifier::PARENT_PORT_KIND,
            Self::MultiscaleSphere => MetaMultiscaleSphereSDFModifier::PARENT_PORT_KIND,
            Self::Union => MetaSDFUnion::PARENT_PORT_KIND,
            Self::Subtraction => MetaSDFSubtraction::PARENT_PORT_KIND,
            Self::Intersection => MetaSDFIntersection::PARENT_PORT_KIND,
            Self::GroupUnion => MetaSDFGroupUnion::PARENT_PORT_KIND,
            Self::StratifiedPlacement => MetaStratifiedPlacement::PARENT_PORT_KIND,
            Self::TranslationToSurface => MetaTranslationToSurface::PARENT_PORT_KIND,
            Self::RotationToGradient => MetaRotationToGradient::PARENT_PORT_KIND,
            Self::Scattering => MetaSDFScattering::PARENT_PORT_KIND,
            Self::StochasticSelection => MetaStochasticSelection::PARENT_PORT_KIND,
        }
    }

    const fn raw_child_port_kinds(&self) -> MetaChildPortKinds {
        match self {
            Self::Output => single_child_port_kind(MetaChildPortKind::SingleSDF),
            Self::Box => MetaBoxSDF::CHILD_PORT_KINDS,
            Self::Sphere => MetaSphereSDF::CHILD_PORT_KINDS,
            Self::GradientNoise => MetaGradientNoiseSDF::CHILD_PORT_KINDS,
            Self::Translation => MetaSDFTranslation::CHILD_PORT_KINDS,
            Self::Rotation => MetaSDFRotation::CHILD_PORT_KINDS,
            Self::Scaling => MetaSDFScaling::CHILD_PORT_KINDS,
            Self::MultifractalNoise => MetaMultifractalNoiseSDFModifier::CHILD_PORT_KINDS,
            Self::MultiscaleSphere => MetaMultiscaleSphereSDFModifier::CHILD_PORT_KINDS,
            Self::Union => MetaSDFUnion::CHILD_PORT_KINDS,
            Self::Subtraction => MetaSDFSubtraction::CHILD_PORT_KINDS,
            Self::Intersection => MetaSDFIntersection::CHILD_PORT_KINDS,
            Self::GroupUnion => MetaSDFGroupUnion::CHILD_PORT_KINDS,
            Self::StratifiedPlacement => MetaStratifiedPlacement::CHILD_PORT_KINDS,
            Self::TranslationToSurface => MetaTranslationToSurface::CHILD_PORT_KINDS,
            Self::RotationToGradient => MetaRotationToGradient::CHILD_PORT_KINDS,
            Self::Scattering => MetaSDFScattering::CHILD_PORT_KINDS,
            Self::StochasticSelection => MetaStochasticSelection::CHILD_PORT_KINDS,
        }
    }

    pub fn child_port_kinds(&self) -> impl Iterator<Item = MetaChildPortKind> {
        self.raw_child_port_kinds().into_iter().flatten()
    }

    pub const fn n_child_slots(&self) -> usize {
        match self.raw_child_port_kinds() {
            [None, None] => 0,
            [Some(_), None] | [None, Some(_)] => 1,
            [Some(_), Some(_)] => 2,
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

const fn leaf_child_port_kind() -> MetaChildPortKinds {
    [None; MAX_CHILD_PORTS]
}

const fn single_child_port_kind(kind: MetaChildPortKind) -> MetaChildPortKinds {
    [Some(kind), None]
}

const fn two_child_port_kinds(
    kind_1: MetaChildPortKind,
    kind_2: MetaChildPortKind,
) -> MetaChildPortKinds {
    [Some(kind_1), Some(kind_2)]
}

pub fn get_voxel_extent_and_seed_from_output_node(
    output_node_params: &[MetaNodeParam],
) -> (f32, u32) {
    (output_node_params[0].float(), output_node_params[1].uint())
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
    params.push(MetaUIntParam::new(LabelAndHoverText::label_only("Seed"), 0).into());
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
