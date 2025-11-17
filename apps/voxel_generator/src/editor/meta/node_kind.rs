use super::{
    MetaEnumParam, MetaFloatParam, MetaFloatRangeParam, MetaNodeID, MetaNodeLink, MetaNodeParam,
    MetaNodeParams, MetaUIntParam, MetaUIntRangeParam,
};
use impact::impact_containers::HashMap;
use impact_dev_ui::option_panels::LabelAndHoverText;
use impact_voxel::generation::sdf::meta::{
    CompositionMode, MetaBoxSDF, MetaCapsuleSDF, MetaGradientNoiseSDF,
    MetaMultifractalNoiseSDFModifier, MetaMultiscaleSphereSDFModifier, MetaRotationToGradient,
    MetaSDFGroupUnion, MetaSDFIntersection, MetaSDFNode, MetaSDFNodeID, MetaSDFRotation,
    MetaSDFScaling, MetaSDFSubtraction, MetaSDFTranslation, MetaSDFUnion, MetaSphereSDF,
    MetaSphereSurfaceTransforms, MetaStochasticSelection, MetaStratifiedGridTransforms,
    MetaTransformApplication, MetaTransformRotation, MetaTransformScaling,
    MetaTransformTranslation, MetaTranslationToSurface, SphereSurfaceRotation,
};
use serde::{Deserialize, Serialize};

trait SpecificMetaNodeKind {
    const LABEL: LabelAndHoverText;
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
    BoxSDF,
    SphereSDF,
    CapsuleSDF,
    GradientNoiseSDF,
    SDFTranslation,
    SDFRotation,
    SDFScaling,
    MultifractalNoiseSDFModifier,
    MultiscaleSphereSDFModifier,
    SDFUnion,
    SDFSubtraction,
    SDFIntersection,
    SDFGroupUnion,
    StratifiedGridTransforms,
    SphereSurfaceTransforms,
    TransformTranslation,
    TransformRotation,
    TransformScaling,
    TranslationToSurface,
    RotationToGradient,
    TransformApplication,
    StochasticSelection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetaNodeKindGroup {
    Root,
    SDFPrimitive,
    SDFTransform,
    SDFModification,
    SDFCombination,
    Transform,
    Filtering,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MetaChildPortKind {
    #[default]
    SingleSDF,
    SDFGroup,
    SingleTransform,
    TransformGroup,
    Any,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MetaParentPortKind {
    #[default]
    SingleSDF,
    SDFGroup,
    SingleTransform,
    TransformGroup,
    SameAsInput {
        slot: usize,
    },
}

const MAX_CHILD_PORTS: usize = 2;

type MetaChildPortKinds = [Option<MetaChildPortKind>; MAX_CHILD_PORTS];

pub const DEFAULT_VOXEL_EXTENT: f32 = 0.25;
pub const MIN_VOXEL_EXTENT: f32 = 0.005;

impl SpecificMetaNodeKind for MetaBoxSDF {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Box SDF",
        hover_text: "A box-shaped SDF.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SingleSDF;
    const CHILD_PORT_KINDS: MetaChildPortKinds = leaf_child_port_kind();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Extent x",
                    hover_text: "Extent of the box along the x-axis, in voxels.",
                },
                60.0,
            )
            .with_min_value(0.0)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Extent y",
                    hover_text: "Extent of the box along the y-axis, in voxels.",
                },
                60.0,
            )
            .with_min_value(0.0)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Extent z",
                    hover_text: "Extent of the box along the z-axis, in voxels.",
                },
                60.0,
            )
            .with_min_value(0.0)
            .into(),
        );
        params.push(
            MetaUIntParam::new(
                LabelAndHoverText {
                    label: "Seed",
                    hover_text: "Seed for selecting an extent within the specified ranges.",
                },
                0,
            )
            .into(),
        );
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
        Some(MetaSDFNode::new_box_sdf(extents, seed))
    }
}

impl SpecificMetaNodeKind for MetaSphereSDF {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Sphere SDF",
        hover_text: "A sphere-shaped SDF.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SingleSDF;
    const CHILD_PORT_KINDS: MetaChildPortKinds = leaf_child_port_kind();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Radius",
                    hover_text: "Radius of the sphere, in voxels.",
                },
                30.0,
            )
            .with_min_value(0.0)
            .into(),
        );
        params.push(
            MetaUIntParam::new(
                LabelAndHoverText {
                    label: "Seed",
                    hover_text: "Seed for selecting a radius within the specified range.",
                },
                0,
            )
            .into(),
        );
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
        Some(MetaSDFNode::new_sphere_sdf(radius, seed))
    }
}

impl SpecificMetaNodeKind for MetaCapsuleSDF {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Capsule SDF",
        hover_text: "A vertical capsule-shaped SDF.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SingleSDF;
    const CHILD_PORT_KINDS: MetaChildPortKinds = leaf_child_port_kind();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Segment length",
                    hover_text: "Length between the centers of the spherical caps, in voxels.",
                },
                30.0,
            )
            .with_min_value(0.0)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Radius",
                    hover_text: "Radius of the spherical caps, in voxels.",
                },
                15.0,
            )
            .with_min_value(0.0)
            .into(),
        );
        params.push(
            MetaUIntParam::new(
                LabelAndHoverText {
                    label: "Seed",
                    hover_text: "Seed for selecting a segment length radius within the specified ranges.",
                },
                0,
            )
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
        let segment_length = params[0].float_range();
        let radius = params[1].float_range();
        let seed = params[2].uint();
        Some(MetaSDFNode::new_capsule_sdf(segment_length, radius, seed))
    }
}

impl SpecificMetaNodeKind for MetaGradientNoiseSDF {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Gradient noise SDF",
        hover_text: "An SDF generated from thresholding a gradient noise field.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SingleSDF;
    const CHILD_PORT_KINDS: MetaChildPortKinds = leaf_child_port_kind();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Extent x",
                    hover_text: "Extent of the noise field along the x-axis, in voxels.",
                },
                60.0,
            )
            .with_min_value(0.0)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Extent y",
                    hover_text: "Extent of the noise field along the y-axis, in voxels.",
                },
                60.0,
            )
            .with_min_value(0.0)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Extent z",
                    hover_text: "Extent of the noise field along the z-axis, in voxels.",
                },
                60.0,
            )
            .with_min_value(0.0)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Frequency",
                    hover_text: "Spatial frequency of the noise pattern, in inverse voxels.",
                },
                0.05,
            )
            .with_min_value(0.0)
            .with_max_value(1.0)
            .with_speed(0.0002)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Threshold",
                    hover_text: "Minimum noise value (they range from -1 to 1) for a voxel to be considered inside the object.",
                },
                0.0,
            )
            .with_min_value(-1.0)
            .with_max_value(1.0)
            .with_speed(0.001)
            .into(),
        );
        params.push(
            MetaUIntParam::new(
                LabelAndHoverText {
                    label: "Seed",
                    hover_text: "Seed for generating noise and selecting parameter values within the specified ranges.",
                },
                0,
            )
            .into(),
        );
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
        Some(MetaSDFNode::new_gradient_noise_sdf(
            extents,
            noise_frequency,
            noise_threshold,
            seed,
        ))
    }
}

impl SpecificMetaNodeKind for MetaSDFTranslation {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "SDF translation",
        hover_text: "Translation of one or more SDFs.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SameAsInput { slot: 0 };
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        single_child_port_kind(MetaChildPortKind::SDFGroup);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "In x",
                    hover_text: "Translation distance along the x-axis, in voxels.",
                },
                0.0,
            )
            .with_speed(0.05)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "In y",
                    hover_text: "Translation distance along the y-axis, in voxels.",
                },
                0.0,
            )
            .with_speed(0.05)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "In z",
                    hover_text: "Translation distance along the z-axis, in voxels.",
                },
                0.0,
            )
            .with_speed(0.05)
            .into(),
        );
        params.push(
            MetaUIntParam::new(
                LabelAndHoverText {
                    label: "Seed",
                    hover_text: "Seed for selecting a translation within the specified ranges.",
                },
                0,
            )
            .into(),
        );
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
        Some(MetaSDFNode::new_sdf_translation(
            child_id,
            translation,
            seed,
        ))
    }
}

impl SpecificMetaNodeKind for MetaSDFRotation {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "SDF rotation",
        hover_text: "Rotation of one or more SDFs.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SameAsInput { slot: 0 };
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        single_child_port_kind(MetaChildPortKind::SDFGroup);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Roll",
                    hover_text: "Rotation angle around the x-axis, in radians.",
                },
                0.0,
            )
            .with_speed(0.002)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Pitch",
                    hover_text: "Rotation angle around the y-axis, in radians.",
                },
                0.0,
            )
            .with_speed(0.002)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Yaw",
                    hover_text: "Rotation angle around the z-axis, in radians.",
                },
                0.0,
            )
            .with_speed(0.002)
            .into(),
        );
        params.push(
            MetaUIntParam::new(
                LabelAndHoverText {
                    label: "Seed",
                    hover_text: "Seed for selecting a rotation within the specified ranges.",
                },
                0,
            )
            .into(),
        );
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
        Some(MetaSDFNode::new_sdf_rotation(
            child_id, roll, pitch, yaw, seed,
        ))
    }
}

impl SpecificMetaNodeKind for MetaSDFScaling {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "SDF scaling",
        hover_text: "Uniform scaling of one or more SDFs.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SameAsInput { slot: 0 };
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        single_child_port_kind(MetaChildPortKind::SDFGroup);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Factor",
                    hover_text: "Uniform scale factor.",
                },
                1.0,
            )
            .with_min_value(1e-3)
            .with_speed(0.005)
            .into(),
        );
        params.push(
            MetaUIntParam::new(
                LabelAndHoverText {
                    label: "Seed",
                    hover_text: "Seed for selecting a scale factor within the specified range.",
                },
                0,
            )
            .into(),
        );
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
        Some(MetaSDFNode::new_sdf_scaling(child_id, scaling, seed))
    }
}

impl SpecificMetaNodeKind for MetaMultifractalNoiseSDFModifier {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Multifractal noise SDF modifier",
        hover_text: "Perturbation of one or more SDFs using a multifractal noise field.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SameAsInput { slot: 0 };
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        single_child_port_kind(MetaChildPortKind::SDFGroup);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaUIntRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Octaves",
                    hover_text: "Number of noise octaves (patterns of increasing frequency) to combine.",
                },
                1,
            )
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Frequency",
                    hover_text: "Spatial frequency of the noise pattern in the first octave, in inverse voxels.",
                },
                0.02,
            )
            .with_min_value(0.0)
            .with_max_value(1.0)
            .with_speed(0.0002)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Lacunarity",
                    hover_text: "Noise frequency multiplier between successive octaves.",
                },
                2.0,
            )
            .with_min_value(1.0)
            .with_max_value(10.0)
            .with_speed(0.001)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Persistence",
                    hover_text: "Noise amplitude multiplier between successive octaves.",
                },
                0.5,
            )
            .with_min_value(0.0)
            .with_max_value(1.0)
            .with_speed(0.001)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Amplitude",
                    hover_text: "Noise amplitude (max displacement) in the first octave, in voxels.",
                },
                5.0,
            )
            .with_min_value(0.0)
            .with_speed(0.05)
            .into(),
        );
        params.push(
            MetaUIntParam::new(
                LabelAndHoverText {
                    label: "Seed",
                    hover_text: "Seed for generating noise and selecting parameter values within the specified ranges.",
                },
                0,
            )
            .into(),
        );
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
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Multiscale sphere SDF modifier",
        hover_text: "Perturbation of one or more SDFs by intersecting and combining with grids of spheres on multiple scales.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SameAsInput { slot: 0 };
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        single_child_port_kind(MetaChildPortKind::SDFGroup);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaUIntRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Octaves",
                    hover_text: "Number of sphere scales to combine for detail variation.",
                },
                0,
            )
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Max scale",
                    hover_text: "Maximum scale of variation in the multiscale pattern, in voxels.",
                },
                10.0,
            )
            .with_min_value(0.0)
            .with_speed(0.01)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Persistence",
                    hover_text: "Scale multiplier between successive octaves.",
                },
                0.5,
            )
            .with_min_value(0.0)
            .with_max_value(1.0)
            .with_speed(0.001)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Inflation",
                    hover_text: "Amount to expand the pattern being modified before intersecting with spheres, in factors of the max scale.",
                },
                1.0,
            )
            .with_min_value(0.0)
            .with_speed(0.005)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Intersection smoothness",
                    hover_text: "Smoothness factor for intersecting spheres with the inflated version of the pattern being modified.",
                },
                1.0,
            )
            .with_min_value(0.0)
            .with_speed(0.001)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Union smoothness",
                    hover_text: "Smoothness factor for combining the intersected sphere pattern with the original pattern.",
                },
                0.3,
            )
            .with_min_value(0.0)
            .with_speed(0.001)
            .into(),
        );
        params.push(
            MetaUIntParam::new(
                LabelAndHoverText {
                    label: "Seed",
                    hover_text: "Seed for generating random sphere radii as well as selecting parameter values within the specified ranges..",
                },
                0,
            )
            .into(),
        );
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
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "SDF union",
        hover_text: "Smooth union of two SDFs.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SingleSDF;
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        two_child_port_kinds(MetaChildPortKind::SingleSDF, MetaChildPortKind::SingleSDF);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatParam::new(
                LabelAndHoverText {
                    label: "Smoothness",
                    hover_text: "Smoothness factor for blending the two shapes together.",
                },
                1.0,
            )
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
        Some(MetaSDFNode::new_sdf_union(
            child_1_id, child_2_id, smoothness,
        ))
    }
}

impl SpecificMetaNodeKind for MetaSDFSubtraction {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "SDF subtraction",
        hover_text: "Smooth subtraction of the second SDF from the first.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SingleSDF;
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        two_child_port_kinds(MetaChildPortKind::SingleSDF, MetaChildPortKind::SingleSDF);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatParam::new(
                LabelAndHoverText {
                    label: "Smoothness",
                    hover_text: "Smoothness factor for blending the subtraction operation.",
                },
                1.0,
            )
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
        Some(MetaSDFNode::new_sdf_subtraction(
            child_1_id, child_2_id, smoothness,
        ))
    }
}

impl SpecificMetaNodeKind for MetaSDFIntersection {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "SDF intersection",
        hover_text: "Smooth intersection of two SDFs.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SingleSDF;
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        two_child_port_kinds(MetaChildPortKind::SingleSDF, MetaChildPortKind::SingleSDF);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatParam::new(
                LabelAndHoverText {
                    label: "Smoothness",
                    hover_text: "Smoothness factor for blending the intersection operation.",
                },
                1.0,
            )
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
        Some(MetaSDFNode::new_sdf_intersection(
            child_1_id, child_2_id, smoothness,
        ))
    }
}

impl SpecificMetaNodeKind for MetaSDFGroupUnion {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "SDF group union",
        hover_text: "Smooth union of a all SDFs in a group.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SingleSDF;
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        single_child_port_kind(MetaChildPortKind::SDFGroup);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaFloatParam::new(
                LabelAndHoverText {
                    label: "Smoothness",
                    hover_text: "Smoothness factor for blending all the shapes in the group together.",
                },
                1.0,
            )
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
        Some(MetaSDFNode::new_sdf_group_union(child_id, smoothness))
    }
}

impl SpecificMetaNodeKind for MetaStratifiedGridTransforms {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Stratified grid transforms",
        hover_text: "Transforms with translations from the center of a grid to grid points picked by stratified sampling.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::TransformGroup;
    const CHILD_PORT_KINDS: MetaChildPortKinds = leaf_child_port_kind();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaUIntRangeParam::new(
                LabelAndHoverText {
                    label: "Size x",
                    hover_text: "Number of grid cells along the x-axis.",
                },
                1,
                1,
            )
            .into(),
        );
        params.push(
            MetaUIntRangeParam::new(
                LabelAndHoverText {
                    label: "Size y",
                    hover_text: "Number of grid cells along the y-axis.",
                },
                1,
                1,
            )
            .into(),
        );
        params.push(
            MetaUIntRangeParam::new(
                LabelAndHoverText {
                    label: "Size z",
                    hover_text: "Number of grid cells along the z-axis.",
                },
                1,
                1,
            )
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new(
                LabelAndHoverText {
                    label: "Cell extent x",
                    hover_text: "Extent of a grid cell along the x-axis, in voxels.",
                },
                60.0,
                60.0,
            )
            .with_min_value(0.0)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new(
                LabelAndHoverText {
                    label: "Cell extent y",
                    hover_text: "Extent of a grid cell along the y-axis, in voxels.",
                },
                60.0,
                60.0,
            )
            .with_min_value(0.0)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new(
                LabelAndHoverText {
                    label: "Cell extent z",
                    hover_text: "Extent of a grid cell along the z-axis, in voxels.",
                },
                60.0,
                60.0,
            )
            .with_min_value(0.0)
            .into(),
        );
        params.push(
            MetaUIntRangeParam::new(
                LabelAndHoverText {
                    label: "Points per cell",
                    hover_text: "Number of points generated within each grid cell.",
                },
                1,
                1,
            )
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new(
                LabelAndHoverText {
                    label: "Jitter fraction",
                    hover_text: "Fraction of a grid cell to randomly displace the points.",
                },
                0.0,
                0.0,
            )
            .with_min_value(0.0)
            .with_max_value(1.0)
            .with_speed(0.001)
            .into(),
        );
        params.push(
            MetaUIntParam::new(
                LabelAndHoverText {
                    label: "Seed",
                    hover_text: "Seed for random jittering as well as selecting parameter values within the specified ranges.",
                },
                0,
            )
            .into(),
        );
        params
    }

    fn build(
        _id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
        _children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 9);
        let shape = [
            params[0].uint_range(),
            params[1].uint_range(),
            params[2].uint_range(),
        ];
        let cell_extents = [
            params[3].float_range(),
            params[4].float_range(),
            params[5].float_range(),
        ];
        let points_per_grid_cell = params[6].uint_range();
        let jitter_fraction = params[7].float_range();
        let seed = params[8].uint();
        Some(MetaSDFNode::new_stratified_grid_transforms(
            shape,
            cell_extents,
            points_per_grid_cell,
            jitter_fraction,
            seed,
        ))
    }
}

impl SpecificMetaNodeKind for MetaSphereSurfaceTransforms {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Sphere surface transforms",
        hover_text: "Transforms with translations from the center to the surface of a sphere and optional rotations from the y-axis to the radial direction.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::TransformGroup;
    const CHILD_PORT_KINDS: MetaChildPortKinds = leaf_child_port_kind();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaUIntRangeParam::new(
                LabelAndHoverText {
                    label: "Count",
                    hover_text: "Number of transforms to generate.",
                },
                6,
                6,
            )
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new(
                LabelAndHoverText {
                    label: "Radius",
                    hover_text: "Radius of the sphere, in voxels.",
                },
                30.0,
                30.0,
            )
            .with_min_value(0.0)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new(
                LabelAndHoverText {
                    label: "Jitter fraction",
                    hover_text: "Fraction of the regular point spacing to randomly displace the points.",
                },
                0.0,
                0.0,
            )
            .with_min_value(0.0)
            .with_max_value(1.0)
            .with_speed(0.001)
            .into(),
        );
        params.push(
            MetaEnumParam::new(
                LabelAndHoverText {
                    label: "Rotation",
                    hover_text: "Whether to include rotations from the y-axes to the outward radial direction.",
                },
                ["Identity", "Radial"].into(),
                "Identity",
            )
            .into(),
        );
        params.push(
            MetaUIntParam::new(
                LabelAndHoverText {
                    label: "Seed",
                    hover_text: "Seed for random jittering as well as selecting parameter values within the specified ranges.",
                },
                0,
            )
            .into(),
        );
        params
    }

    fn build(
        _id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
        _children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 5);
        let count = params[0].uint_range();
        let radius = params[1].float_range();
        let jitter_fraction = params[2].float_range();
        let rotation = SphereSurfaceRotation::try_from_str(params[3].enum_value()).unwrap();
        let seed = params[4].uint();
        Some(MetaSDFNode::new_sphere_surface_transforms(
            count,
            radius,
            jitter_fraction,
            rotation,
            seed,
        ))
    }
}

impl SpecificMetaNodeKind for MetaTransformTranslation {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Transform translation",
        hover_text: "Translation of one or more transforms.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SameAsInput { slot: 0 };
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        single_child_port_kind(MetaChildPortKind::TransformGroup);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaEnumParam::new(
                LabelAndHoverText {
                    label: "Composition",
                    hover_text: "Whether to apply the translation before ('Pre') or after ('Post') the input transforms.",
                },
                ["Pre", "Post"].into(),
                "Pre",
            )
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "In x",
                    hover_text: "Translation distance along the x-axis, in voxels.",
                },
                0.0,
            )
            .with_speed(0.05)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "In y",
                    hover_text: "Translation distance along the y-axis, in voxels.",
                },
                0.0,
            )
            .with_speed(0.05)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "In z",
                    hover_text: "Translation distance along the z-axis, in voxels.",
                },
                0.0,
            )
            .with_speed(0.05)
            .into(),
        );
        params.push(
            MetaUIntParam::new(
                LabelAndHoverText {
                    label: "Seed",
                    hover_text: "Seed for selecting a translation within the specified ranges.",
                },
                0,
            )
            .into(),
        );
        params
    }

    fn build(
        id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 5);
        let child_id = unary_child(id_map, children)?;
        let composition = CompositionMode::try_from_str(params[0].enum_value()).unwrap();
        let translation = [
            params[1].float_range(),
            params[2].float_range(),
            params[3].float_range(),
        ];
        let seed = params[4].uint();
        Some(MetaSDFNode::new_transform_translation(
            child_id,
            composition,
            translation,
            seed,
        ))
    }
}

impl SpecificMetaNodeKind for MetaTransformRotation {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Transform rotation",
        hover_text: "Rotation of one or more transforms.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SameAsInput { slot: 0 };
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        single_child_port_kind(MetaChildPortKind::TransformGroup);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaEnumParam::new(
                LabelAndHoverText {
                    label: "Composition",
                    hover_text: "Whether to apply the rotation before ('Pre') or after ('Post') the input transforms.",
                },
                ["Pre", "Post"].into(),
                "Pre",
            )
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Roll",
                    hover_text: "Rotation angle around the x-axis, in radians.",
                },
                0.0,
            )
            .with_speed(0.002)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Pitch",
                    hover_text: "Rotation angle around the y-axis, in radians.",
                },
                0.0,
            )
            .with_speed(0.002)
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Yaw",
                    hover_text: "Rotation angle around the z-axis, in radians.",
                },
                0.0,
            )
            .with_speed(0.002)
            .into(),
        );
        params.push(
            MetaUIntParam::new(
                LabelAndHoverText {
                    label: "Seed",
                    hover_text: "Seed for selecting a rotation within the specified ranges.",
                },
                0,
            )
            .into(),
        );
        params
    }

    fn build(
        id_map: &HashMap<MetaNodeID, MetaSDFNodeID>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 5);
        let child_id = unary_child(id_map, children)?;
        let composition = CompositionMode::try_from_str(params[0].enum_value()).unwrap();
        let roll = params[1].float_range();
        let pitch = params[2].float_range();
        let yaw = params[3].float_range();
        let seed = params[4].uint();
        Some(MetaSDFNode::new_transform_rotation(
            child_id,
            composition,
            roll,
            pitch,
            yaw,
            seed,
        ))
    }
}

impl SpecificMetaNodeKind for MetaTransformScaling {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Transform scaling",
        hover_text: "Uniform scaling of one or more transforms.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SameAsInput { slot: 0 };
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        single_child_port_kind(MetaChildPortKind::TransformGroup);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaEnumParam::new(
                LabelAndHoverText {
                    label: "Composition",
                    hover_text: "Whether to apply the scaling before ('Pre') or after ('Post') the input transforms.",
                },
                ["Pre", "Post"].into(),
                "Pre",
            )
            .into(),
        );
        params.push(
            MetaFloatRangeParam::new_single_value(
                LabelAndHoverText {
                    label: "Factor",
                    hover_text: "Uniform scale factor.",
                },
                1.0,
            )
            .with_min_value(1e-3)
            .with_speed(0.005)
            .into(),
        );
        params.push(
            MetaUIntParam::new(
                LabelAndHoverText {
                    label: "Seed",
                    hover_text: "Seed for selecting a scale factor within the specified range.",
                },
                0,
            )
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
        let composition = CompositionMode::try_from_str(params[0].enum_value()).unwrap();
        let scaling = params[1].float_range();
        let seed = params[2].uint();
        Some(MetaSDFNode::new_transform_scaling(
            child_id,
            composition,
            scaling,
            seed,
        ))
    }
}

impl SpecificMetaNodeKind for MetaTranslationToSurface {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Translation to surface",
        hover_text: "Translation of the SDFs or transforms in the second input to the surface of the SDF in the first input.",
    };
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
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Rotation to gradient",
        hover_text: "Rotation of the SDFs or transforms in the second input to make their y-axis align with the gradient of the SDF in the first input.",
    };
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

impl SpecificMetaNodeKind for MetaTransformApplication {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Transform application",
        hover_text: "Application of the transforms in the second input to the SDFs in the first input (yields all combinations).",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SDFGroup;
    const CHILD_PORT_KINDS: MetaChildPortKinds = two_child_port_kinds(
        MetaChildPortKind::SDFGroup,
        MetaChildPortKind::TransformGroup,
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
        let (sdf_id, transform_id) = binary_children(id_map, children)?;
        Some(MetaSDFNode::new_scattering(sdf_id, transform_id))
    }
}

impl SpecificMetaNodeKind for MetaStochasticSelection {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Stochastic selection",
        hover_text: "Random selection of SDFs or transforms from a group.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SameAsInput { slot: 0 };
    const CHILD_PORT_KINDS: MetaChildPortKinds = single_child_port_kind(MetaChildPortKind::Any);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaUIntRangeParam::new(
                LabelAndHoverText {
                    label: "Count",
                    hover_text: "Minimum and maximum number of items to select initially.",
                },
                1,
                1,
            )
            .into(),
        );
        params.push(
            MetaFloatParam::new(
                LabelAndHoverText {
                    label: "Probability",
                    hover_text: "Probability that each of the initially selected items will be kept in the final selection.",
                },
                1.0,
            )
            .with_min_value(0.0)
            .with_max_value(1.0)
            .with_speed(0.001)
            .into(),
        );
        params.push(
            MetaUIntParam::new(
                LabelAndHoverText {
                    label: "Seed",
                    hover_text: "Seed for random selection.",
                },
                0,
            )
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
    pub const fn all_non_root() -> [Self; 22] {
        [
            Self::BoxSDF,
            Self::SphereSDF,
            Self::CapsuleSDF,
            Self::GradientNoiseSDF,
            Self::SDFTranslation,
            Self::SDFRotation,
            Self::SDFScaling,
            Self::MultifractalNoiseSDFModifier,
            Self::MultiscaleSphereSDFModifier,
            Self::SDFUnion,
            Self::SDFSubtraction,
            Self::SDFIntersection,
            Self::SDFGroupUnion,
            Self::StratifiedGridTransforms,
            Self::SphereSurfaceTransforms,
            Self::TransformTranslation,
            Self::TransformRotation,
            Self::TransformScaling,
            Self::TranslationToSurface,
            Self::RotationToGradient,
            Self::TransformApplication,
            Self::StochasticSelection,
        ]
    }

    pub fn is_output(&self) -> bool {
        *self == Self::Output
    }

    pub const fn group(&self) -> MetaNodeKindGroup {
        match self {
            Self::Output => MetaNodeKindGroup::Root,
            Self::BoxSDF | Self::SphereSDF | Self::CapsuleSDF | Self::GradientNoiseSDF => {
                MetaNodeKindGroup::SDFPrimitive
            }
            Self::SDFTranslation | Self::SDFRotation | Self::SDFScaling => {
                MetaNodeKindGroup::SDFTransform
            }
            Self::MultifractalNoiseSDFModifier | Self::MultiscaleSphereSDFModifier => {
                MetaNodeKindGroup::SDFModification
            }
            Self::SDFUnion | Self::SDFSubtraction | Self::SDFIntersection | Self::SDFGroupUnion => {
                MetaNodeKindGroup::SDFCombination
            }
            Self::StratifiedGridTransforms
            | Self::SphereSurfaceTransforms
            | Self::TransformTranslation
            | Self::TransformRotation
            | Self::TransformScaling
            | Self::TranslationToSurface
            | Self::RotationToGradient
            | Self::TransformApplication => MetaNodeKindGroup::Transform,
            Self::StochasticSelection => MetaNodeKindGroup::Filtering,
        }
    }

    pub const fn label(&self) -> LabelAndHoverText {
        match self {
            Self::Output => LabelAndHoverText::label_only("Output"),
            Self::BoxSDF => MetaBoxSDF::LABEL,
            Self::SphereSDF => MetaSphereSDF::LABEL,
            Self::CapsuleSDF => MetaCapsuleSDF::LABEL,
            Self::GradientNoiseSDF => MetaGradientNoiseSDF::LABEL,
            Self::SDFTranslation => MetaSDFTranslation::LABEL,
            Self::SDFRotation => MetaSDFRotation::LABEL,
            Self::SDFScaling => MetaSDFScaling::LABEL,
            Self::MultifractalNoiseSDFModifier => MetaMultifractalNoiseSDFModifier::LABEL,
            Self::MultiscaleSphereSDFModifier => MetaMultiscaleSphereSDFModifier::LABEL,
            Self::SDFUnion => MetaSDFUnion::LABEL,
            Self::SDFSubtraction => MetaSDFSubtraction::LABEL,
            Self::SDFIntersection => MetaSDFIntersection::LABEL,
            Self::SDFGroupUnion => MetaSDFGroupUnion::LABEL,
            Self::StratifiedGridTransforms => MetaStratifiedGridTransforms::LABEL,
            Self::SphereSurfaceTransforms => MetaSphereSurfaceTransforms::LABEL,
            Self::TransformTranslation => MetaTransformTranslation::LABEL,
            Self::TransformRotation => MetaTransformRotation::LABEL,
            Self::TransformScaling => MetaTransformScaling::LABEL,
            Self::TranslationToSurface => MetaTranslationToSurface::LABEL,
            Self::RotationToGradient => MetaRotationToGradient::LABEL,
            Self::TransformApplication => MetaTransformApplication::LABEL,
            Self::StochasticSelection => MetaStochasticSelection::LABEL,
        }
    }

    pub const fn parent_port_kind(&self) -> MetaParentPortKind {
        match self {
            Self::Output => MetaParentPortKind::SingleSDF,
            Self::BoxSDF => MetaBoxSDF::PARENT_PORT_KIND,
            Self::SphereSDF => MetaSphereSDF::PARENT_PORT_KIND,
            Self::CapsuleSDF => MetaCapsuleSDF::PARENT_PORT_KIND,
            Self::GradientNoiseSDF => MetaGradientNoiseSDF::PARENT_PORT_KIND,
            Self::SDFTranslation => MetaSDFTranslation::PARENT_PORT_KIND,
            Self::SDFRotation => MetaSDFRotation::PARENT_PORT_KIND,
            Self::SDFScaling => MetaSDFScaling::PARENT_PORT_KIND,
            Self::MultifractalNoiseSDFModifier => {
                MetaMultifractalNoiseSDFModifier::PARENT_PORT_KIND
            }
            Self::MultiscaleSphereSDFModifier => MetaMultiscaleSphereSDFModifier::PARENT_PORT_KIND,
            Self::SDFUnion => MetaSDFUnion::PARENT_PORT_KIND,
            Self::SDFSubtraction => MetaSDFSubtraction::PARENT_PORT_KIND,
            Self::SDFIntersection => MetaSDFIntersection::PARENT_PORT_KIND,
            Self::SDFGroupUnion => MetaSDFGroupUnion::PARENT_PORT_KIND,
            Self::StratifiedGridTransforms => MetaStratifiedGridTransforms::PARENT_PORT_KIND,
            Self::SphereSurfaceTransforms => MetaSphereSurfaceTransforms::PARENT_PORT_KIND,
            Self::TransformTranslation => MetaTransformTranslation::PARENT_PORT_KIND,
            Self::TransformRotation => MetaTransformRotation::PARENT_PORT_KIND,
            Self::TransformScaling => MetaTransformScaling::PARENT_PORT_KIND,
            Self::TranslationToSurface => MetaTranslationToSurface::PARENT_PORT_KIND,
            Self::RotationToGradient => MetaRotationToGradient::PARENT_PORT_KIND,
            Self::TransformApplication => MetaTransformApplication::PARENT_PORT_KIND,
            Self::StochasticSelection => MetaStochasticSelection::PARENT_PORT_KIND,
        }
    }

    const fn raw_child_port_kinds(&self) -> MetaChildPortKinds {
        match self {
            Self::Output => single_child_port_kind(MetaChildPortKind::SingleSDF),
            Self::BoxSDF => MetaBoxSDF::CHILD_PORT_KINDS,
            Self::SphereSDF => MetaSphereSDF::CHILD_PORT_KINDS,
            Self::CapsuleSDF => MetaCapsuleSDF::CHILD_PORT_KINDS,
            Self::GradientNoiseSDF => MetaGradientNoiseSDF::CHILD_PORT_KINDS,
            Self::SDFTranslation => MetaSDFTranslation::CHILD_PORT_KINDS,
            Self::SDFRotation => MetaSDFRotation::CHILD_PORT_KINDS,
            Self::SDFScaling => MetaSDFScaling::CHILD_PORT_KINDS,
            Self::MultifractalNoiseSDFModifier => {
                MetaMultifractalNoiseSDFModifier::CHILD_PORT_KINDS
            }
            Self::MultiscaleSphereSDFModifier => MetaMultiscaleSphereSDFModifier::CHILD_PORT_KINDS,
            Self::SDFUnion => MetaSDFUnion::CHILD_PORT_KINDS,
            Self::SDFSubtraction => MetaSDFSubtraction::CHILD_PORT_KINDS,
            Self::SDFIntersection => MetaSDFIntersection::CHILD_PORT_KINDS,
            Self::SDFGroupUnion => MetaSDFGroupUnion::CHILD_PORT_KINDS,
            Self::StratifiedGridTransforms => MetaStratifiedGridTransforms::CHILD_PORT_KINDS,
            Self::SphereSurfaceTransforms => MetaSphereSurfaceTransforms::CHILD_PORT_KINDS,
            Self::TransformTranslation => MetaTransformTranslation::CHILD_PORT_KINDS,
            Self::TransformRotation => MetaTransformRotation::CHILD_PORT_KINDS,
            Self::TransformScaling => MetaTransformScaling::CHILD_PORT_KINDS,
            Self::TranslationToSurface => MetaTranslationToSurface::CHILD_PORT_KINDS,
            Self::RotationToGradient => MetaRotationToGradient::CHILD_PORT_KINDS,
            Self::TransformApplication => MetaTransformApplication::CHILD_PORT_KINDS,
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
            Self::BoxSDF => MetaBoxSDF::params(),
            Self::SphereSDF => MetaSphereSDF::params(),
            Self::CapsuleSDF => MetaCapsuleSDF::params(),
            Self::GradientNoiseSDF => MetaGradientNoiseSDF::params(),
            Self::SDFTranslation => MetaSDFTranslation::params(),
            Self::SDFRotation => MetaSDFRotation::params(),
            Self::SDFScaling => MetaSDFScaling::params(),
            Self::MultifractalNoiseSDFModifier => MetaMultifractalNoiseSDFModifier::params(),
            Self::MultiscaleSphereSDFModifier => MetaMultiscaleSphereSDFModifier::params(),
            Self::SDFUnion => MetaSDFUnion::params(),
            Self::SDFSubtraction => MetaSDFSubtraction::params(),
            Self::SDFIntersection => MetaSDFIntersection::params(),
            Self::SDFGroupUnion => MetaSDFGroupUnion::params(),
            Self::StratifiedGridTransforms => MetaStratifiedGridTransforms::params(),
            Self::SphereSurfaceTransforms => MetaSphereSurfaceTransforms::params(),
            Self::TransformTranslation => MetaTransformTranslation::params(),
            Self::TransformRotation => MetaTransformRotation::params(),
            Self::TransformScaling => MetaTransformScaling::params(),
            Self::TranslationToSurface => MetaTranslationToSurface::params(),
            Self::RotationToGradient => MetaRotationToGradient::params(),
            Self::TransformApplication => MetaTransformApplication::params(),
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
            Self::BoxSDF => MetaBoxSDF::build(id_map, children, params),
            Self::SphereSDF => MetaSphereSDF::build(id_map, children, params),
            Self::CapsuleSDF => MetaCapsuleSDF::build(id_map, children, params),
            Self::GradientNoiseSDF => MetaGradientNoiseSDF::build(id_map, children, params),
            Self::SDFTranslation => MetaSDFTranslation::build(id_map, children, params),
            Self::SDFRotation => MetaSDFRotation::build(id_map, children, params),
            Self::SDFScaling => MetaSDFScaling::build(id_map, children, params),
            Self::MultifractalNoiseSDFModifier => {
                MetaMultifractalNoiseSDFModifier::build(id_map, children, params)
            }
            Self::MultiscaleSphereSDFModifier => {
                MetaMultiscaleSphereSDFModifier::build(id_map, children, params)
            }
            Self::SDFUnion => MetaSDFUnion::build(id_map, children, params),
            Self::SDFSubtraction => MetaSDFSubtraction::build(id_map, children, params),
            Self::SDFIntersection => MetaSDFIntersection::build(id_map, children, params),
            Self::SDFGroupUnion => MetaSDFGroupUnion::build(id_map, children, params),
            Self::StratifiedGridTransforms => {
                MetaStratifiedGridTransforms::build(id_map, children, params)
            }
            Self::SphereSurfaceTransforms => {
                MetaSphereSurfaceTransforms::build(id_map, children, params)
            }
            Self::TransformTranslation => MetaTransformTranslation::build(id_map, children, params),
            Self::TransformRotation => MetaTransformRotation::build(id_map, children, params),
            Self::TransformScaling => MetaTransformScaling::build(id_map, children, params),
            Self::TranslationToSurface => MetaTranslationToSurface::build(id_map, children, params),
            Self::RotationToGradient => MetaRotationToGradient::build(id_map, children, params),
            Self::TransformApplication => MetaTransformApplication::build(id_map, children, params),
            Self::StochasticSelection => MetaStochasticSelection::build(id_map, children, params),
        }
    }
}

impl MetaNodeKindGroup {
    pub const fn all_non_root() -> [Self; 6] {
        [
            Self::SDFPrimitive,
            Self::SDFTransform,
            Self::SDFModification,
            Self::SDFCombination,
            Self::Transform,
            Self::Filtering,
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
            LabelAndHoverText {
                label: "Voxel extent",
                hover_text: "The size of each voxel in the generated voxel grid.",
            },
            DEFAULT_VOXEL_EXTENT,
        )
        .with_min_value(MIN_VOXEL_EXTENT)
        .with_speed(0.01)
        .into(),
    );
    params.push(
        MetaUIntParam::new(
            LabelAndHoverText {
                label: "Seed",
                hover_text: "Global seed offset added to the seed of all nodes.",
            },
            0,
        )
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
