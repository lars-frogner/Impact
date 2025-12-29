use super::{
    MetaNodeID, MetaNodeLink,
    param::{
        EnumParamVariants, MetaDistributedParam, MetaEnumParam, MetaFloatParam, MetaNodeParam,
        MetaNodeParams, MetaUIntParam,
    },
};
use impact::impact_alloc::Allocator;
use impact_containers::NoHashMap;
use impact_dev_ui::option_panels::LabelAndHoverText;
use impact_voxel::generation::sdf::meta::{
    CompositionMode, MetaBoxes, MetaCapsules, MetaClosestTranslationToSurface,
    MetaMultifractalNoiseSDFModifier, MetaPoints, MetaRayTranslationToSurface, MetaRotation,
    MetaRotationToGradient, MetaSDFGroupUnion, MetaSDFInstantiation, MetaSDFIntersection,
    MetaSDFNode, MetaSDFNodeID, MetaSDFSubtraction, MetaSDFUnion, MetaScaling, MetaSimilarity,
    MetaSphereSurfaceTransforms, MetaSpheres, MetaStochasticSelection,
    MetaStratifiedGridTransforms, MetaTransformApplication, MetaTranslation, ParameterSamplingMode,
    RayTranslationAnchor, SphereSurfaceRotation,
};
use serde::{Deserialize, Serialize};

trait SpecificMetaNodeKind {
    const LABEL: LabelAndHoverText;
    const PARENT_PORT_KIND: MetaParentPortKind;
    const CHILD_PORT_KINDS: MetaChildPortKinds;

    fn params() -> MetaNodeParams;

    fn build<A: Allocator>(
        id_map: &NoHashMap<MetaNodeID, MetaSDFNodeID, A>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetaNodeKind {
    Output,
    Points,
    Spheres,
    Capsules,
    Boxes,
    Translation,
    Rotation,
    Scaling,
    Similarity,
    StratifiedGridTransforms,
    SphereSurfaceTransforms,
    ClosestTranslationToSurface,
    RayTranslationToSurface,
    RotationToGradient,
    StochasticSelection,
    SDFInstantiation,
    TransformApplication,
    MultifractalNoiseSDFModifier,
    SDFUnion,
    SDFSubtraction,
    SDFIntersection,
    SDFGroupUnion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetaNodeKindGroup {
    Root,
    InstancePrimitives,
    BasicInstanceTransforms,
    StructuredInstanceTransforms,
    SDFBasedInstanceTransforms,
    Filtering,
    SDFFromInstances,
    SDFModifiers,
    SDFCombination,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MetaChildPortKind {
    #[default]
    SingleSDF,
    SDFGroup,
    Instances,
    Any,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MetaParentPortKind {
    #[default]
    SingleSDF,
    SDFGroup,
    Instances,
    SameAsInput {
        slot: usize,
    },
}

const MAX_CHILD_PORTS: usize = 2;

type MetaChildPortKinds = [Option<MetaChildPortKind>; MAX_CHILD_PORTS];

pub const DEFAULT_VOXEL_EXTENT: f32 = 0.25;
pub const MIN_VOXEL_EXTENT: f32 = 0.005;

impl SpecificMetaNodeKind for MetaPoints {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Points",
        hover_text: "A set of instances with no shape, each having an identity transform.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::Instances;
    const CHILD_PORT_KINDS: MetaChildPortKinds = leaf_child_port_kind();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(MetaUIntParam::new(
            LabelAndHoverText {
                label: "Count",
                hover_text: "Number of points to generate.",
            },
            1,
        ));
        params
    }

    fn build<A: Allocator>(
        _id_map: &NoHashMap<MetaNodeID, MetaSDFNodeID, A>,
        _children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 1);
        Some(MetaSDFNode::Points(MetaPoints {
            count: (&params[0]).into(),
        }))
    }
}

impl SpecificMetaNodeKind for MetaSpheres {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Spheres",
        hover_text: "A set of sphere instances, each having an identity transform.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::Instances;
    const CHILD_PORT_KINDS: MetaChildPortKinds = leaf_child_port_kind();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Radius",
                    hover_text: "Sphere radius, in voxels.",
                },
                30.0,
            )
            .with_min_value(0.0),
        );
        params.push(MetaDistributedParam::new_fixed_constant_continuous_value(
            LabelAndHoverText {
                label: "Center x",
                hover_text: "Sphere center x-coordinate, in voxels.",
            },
            0.0,
        ));
        params.push(MetaDistributedParam::new_fixed_constant_continuous_value(
            LabelAndHoverText {
                label: "Center y",
                hover_text: "Sphere center y-coordinate, in voxels.",
            },
            0.0,
        ));
        params.push(MetaDistributedParam::new_fixed_constant_continuous_value(
            LabelAndHoverText {
                label: "Center z",
                hover_text: "Sphere center z-coordinate, in voxels.",
            },
            0.0,
        ));
        params.push(MetaUIntParam::new(
            LabelAndHoverText {
                label: "Count",
                hover_text: "Number of spheres to generate.",
            },
            1,
        ));
        params.push(MetaUIntParam::new(
            LabelAndHoverText {
                label: "Seed",
                hover_text: "Seed for generating randomized radius values.",
            },
            0,
        ));
        params.push(
            MetaEnumParam::new(
                LabelAndHoverText {
                    label: "Sampling",
                    hover_text: "How to sample parameters from distributions when there are multiple instances.",
                },
                 EnumParamVariants::from_iter(["Only once", "Per instance"]),
                "Only once",
            )
        );
        params
    }

    fn build<A: Allocator>(
        _id_map: &NoHashMap<MetaNodeID, MetaSDFNodeID, A>,
        _children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 7);
        Some(MetaSDFNode::Spheres(MetaSpheres {
            radius: (&params[0]).into(),
            center_x: (&params[1]).into(),
            center_y: (&params[2]).into(),
            center_z: (&params[3]).into(),
            count: (&params[4]).into(),
            seed: (&params[5]).into(),
            sampling: ParameterSamplingMode::try_from_str(params[6].enum_value()).unwrap(),
        }))
    }
}

impl SpecificMetaNodeKind for MetaCapsules {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Capsules",
        hover_text: "A set of vertical capsule instances, each having an identity transform.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::Instances;
    const CHILD_PORT_KINDS: MetaChildPortKinds = leaf_child_port_kind();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Segment length",
                    hover_text: "Length between the centers of the spherical caps, in voxels.",
                },
                30.0,
            )
            .with_min_value(0.0),
        );
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Radius",
                    hover_text: "Radius of the spherical caps, in voxels",
                },
                15.0,
            )
            .with_min_value(0.0),
        );
        params.push(MetaDistributedParam::new_fixed_constant_continuous_value(
            LabelAndHoverText {
                label: "Center x",
                hover_text: "Capsule center x-coordinate, in voxels.",
            },
            0.0,
        ));
        params.push(MetaDistributedParam::new_fixed_constant_continuous_value(
            LabelAndHoverText {
                label: "Center y",
                hover_text: "Capsule center y-coordinate, in voxels.",
            },
            0.0,
        ));
        params.push(MetaDistributedParam::new_fixed_constant_continuous_value(
            LabelAndHoverText {
                label: "Center z",
                hover_text: "Capsule center z-coordinate, in voxels.",
            },
            0.0,
        ));
        params.push(MetaUIntParam::new(
            LabelAndHoverText {
                label: "Count",
                hover_text: "Number of capsules to generate.",
            },
            1,
        ));
        params.push(MetaUIntParam::new(
            LabelAndHoverText {
                label: "Seed",
                hover_text: "Seed for generating randomized segment length and radius values.",
            },
            0,
        ));
        params.push(
            MetaEnumParam::new(
                LabelAndHoverText {
                    label: "Sampling",
                    hover_text: "How to sample parameters from distributions when there are multiple instances.",
                },
                 EnumParamVariants::from_iter(["Only once", "Per instance"]),
                "Only once",
            )
        );
        params
    }

    fn build<A: Allocator>(
        _id_map: &NoHashMap<MetaNodeID, MetaSDFNodeID, A>,
        _children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 8);
        Some(MetaSDFNode::Capsules(MetaCapsules {
            segment_length: (&params[0]).into(),
            radius: (&params[1]).into(),
            center_x: (&params[2]).into(),
            center_y: (&params[3]).into(),
            center_z: (&params[4]).into(),
            count: (&params[5]).into(),
            seed: (&params[6]).into(),
            sampling: ParameterSamplingMode::try_from_str(params[7].enum_value()).unwrap(),
        }))
    }
}

impl SpecificMetaNodeKind for MetaBoxes {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Boxes",
        hover_text: "A set of box instances, each having an identity transform.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::Instances;
    const CHILD_PORT_KINDS: MetaChildPortKinds = leaf_child_port_kind();

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Extent x",
                    hover_text: "Extent along the x-axis, in voxels.",
                },
                60.0,
            )
            .with_min_value(0.0),
        );
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Extent y",
                    hover_text: "Extent along the y-axis, in voxels.",
                },
                60.0,
            )
            .with_min_value(0.0),
        );
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Extent z",
                    hover_text: "Extent along the z-axis, in voxels.",
                },
                60.0,
            )
            .with_min_value(0.0),
        );
        params.push(MetaDistributedParam::new_fixed_constant_continuous_value(
            LabelAndHoverText {
                label: "Center x",
                hover_text: "Box center x-coordinate, in voxels.",
            },
            0.0,
        ));
        params.push(MetaDistributedParam::new_fixed_constant_continuous_value(
            LabelAndHoverText {
                label: "Center y",
                hover_text: "Box center y-coordinate, in voxels.",
            },
            0.0,
        ));
        params.push(MetaDistributedParam::new_fixed_constant_continuous_value(
            LabelAndHoverText {
                label: "Center z",
                hover_text: "Box center z-coordinate, in voxels.",
            },
            0.0,
        ));
        params.push(MetaUIntParam::new(
            LabelAndHoverText {
                label: "Count",
                hover_text: "Number of boxes to generate.",
            },
            1,
        ));
        params.push(MetaUIntParam::new(
            LabelAndHoverText {
                label: "Seed",
                hover_text: "Seed for generating randomized extent values.",
            },
            0,
        ));
        params.push(
            MetaEnumParam::new(
                LabelAndHoverText {
                    label: "Sampling",
                    hover_text: "How to sample parameters from distributions when there are multiple instances.",
                },
                 EnumParamVariants::from_iter(["Only once", "Per instance"]),
                "Only once",
            )
        );
        params
    }

    fn build<A: Allocator>(
        _id_map: &NoHashMap<MetaNodeID, MetaSDFNodeID, A>,
        _children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 9);
        Some(MetaSDFNode::Boxes(MetaBoxes {
            extent_x: (&params[0]).into(),
            extent_y: (&params[1]).into(),
            extent_z: (&params[2]).into(),
            center_x: (&params[3]).into(),
            center_y: (&params[4]).into(),
            center_z: (&params[5]).into(),
            count: (&params[6]).into(),
            seed: (&params[7]).into(),
            sampling: ParameterSamplingMode::try_from_str(params[8].enum_value()).unwrap(),
        }))
    }
}

impl SpecificMetaNodeKind for MetaTranslation {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Translation",
        hover_text: "Translation of one or more instances.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::Instances;
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        single_child_port_kind(MetaChildPortKind::Instances);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaEnumParam::new(
                LabelAndHoverText {
                    label: "Composition",
                    hover_text: "Whether to apply the translation after ('Post') or before ('Pre') the transforms of the input instances.",
                },
                 EnumParamVariants::from_iter(["Post", "Pre"]),
                "Post",
            )
        );
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "In x",
                    hover_text: "Translation distance along the x-axis, in voxels.",
                },
                0.0,
            )
            .with_speed(0.05),
        );
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "In y",
                    hover_text: "Translation distance along the y-axis, in voxels.",
                },
                0.0,
            )
            .with_speed(0.05),
        );
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "In z",
                    hover_text: "Translation distance along the z-axis, in voxels.",
                },
                0.0,
            )
            .with_speed(0.05),
        );
        params.push(MetaUIntParam::new(
            LabelAndHoverText {
                label: "Seed",
                hover_text: "Seed for generating randomized translations.",
            },
            0,
        ));
        params.push(
            MetaEnumParam::new(
                LabelAndHoverText {
                    label: "Sampling",
                    hover_text: "How to sample parameters from distributions when there are multiple instances.",
                },
                 EnumParamVariants::from_iter(["Only once", "Per instance"]),
                "Only once",
            )
        );
        params
    }

    fn build<A: Allocator>(
        id_map: &NoHashMap<MetaNodeID, MetaSDFNodeID, A>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 6);
        Some(MetaSDFNode::Translation(MetaTranslation {
            child_id: unary_child(id_map, children)?,
            composition: CompositionMode::try_from_str(params[0].enum_value()).unwrap(),
            translation_x: (&params[1]).into(),
            translation_y: (&params[2]).into(),
            translation_z: (&params[3]).into(),
            seed: (&params[4]).into(),
            sampling: ParameterSamplingMode::try_from_str(params[5].enum_value()).unwrap(),
        }))
    }
}

impl SpecificMetaNodeKind for MetaRotation {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Rotation",
        hover_text: "Rotation of one or more instances.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::Instances;
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        single_child_port_kind(MetaChildPortKind::Instances);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaEnumParam::new(
                LabelAndHoverText {
                    label: "Composition",
                    hover_text: "Whether to apply the rotation after ('Post') or before ('Pre') the transforms of the input instances.",
                },
                 EnumParamVariants::from_iter(["Post", "Pre"]),
                "Post",
            )
        );
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Tilt angle",
                    hover_text: "Angle away from the y-axis, in degrees",
                },
                0.0,
            )
            .with_speed(0.03),
        );
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Turn angle",
                    hover_text: "Angle from the x-axis in the xz-plane, in degrees.",
                },
                0.0,
            )
            .with_speed(0.03),
        );
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Roll angle",
                    hover_text: "Additional roll angle around the final rotated axis, in degrees.",
                },
                0.0,
            )
            .with_speed(0.03),
        );
        params.push(MetaUIntParam::new(
            LabelAndHoverText {
                label: "Seed",
                hover_text: "Seed for generating randomized rotations.",
            },
            0,
        ));
        params.push(
            MetaEnumParam::new(
                LabelAndHoverText {
                    label: "Sampling",
                    hover_text: "How to sample parameters from distributions when there are multiple instances.",
                },
                 EnumParamVariants::from_iter(["Only once", "Per instance"]),
                "Only once",
            )
        );
        params
    }

    fn build<A: Allocator>(
        id_map: &NoHashMap<MetaNodeID, MetaSDFNodeID, A>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 6);
        Some(MetaSDFNode::Rotation(MetaRotation {
            child_id: unary_child(id_map, children)?,
            composition: CompositionMode::try_from_str(params[0].enum_value()).unwrap(),
            tilt_angle: (&params[1]).into(),
            turn_angle: (&params[2]).into(),
            roll_angle: (&params[3]).into(),
            seed: (&params[4]).into(),
            sampling: ParameterSamplingMode::try_from_str(params[5].enum_value()).unwrap(),
        }))
    }
}

impl SpecificMetaNodeKind for MetaScaling {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Scaling",
        hover_text: "Uniform scaling of one or more instances.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::Instances;
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        single_child_port_kind(MetaChildPortKind::Instances);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaEnumParam::new(
                LabelAndHoverText {
                    label: "Composition",
                    hover_text: "Whether to apply the scaling after ('Post') or before ('Pre') the transforms of the input instances.",
                },
                 EnumParamVariants::from_iter(["Post", "Pre"]),
                "Post",
            )
        );
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Factor",
                    hover_text: "Uniform scale factor.",
                },
                1.0,
            )
            .with_min_value(1e-3)
            .with_speed(0.005),
        );
        params.push(MetaUIntParam::new(
            LabelAndHoverText {
                label: "Seed",
                hover_text: "Seed for generating randomized scale factors.",
            },
            0,
        ));
        params.push(
            MetaEnumParam::new(
                LabelAndHoverText {
                    label: "Sampling",
                    hover_text: "How to sample parameters from distributions when there are multiple instances.",
                },
                 EnumParamVariants::from_iter(["Only once", "Per instance"]),
                "Only once",
            )
        );
        params
    }

    fn build<A: Allocator>(
        id_map: &NoHashMap<MetaNodeID, MetaSDFNodeID, A>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 4);
        Some(MetaSDFNode::Scaling(MetaScaling {
            child_id: unary_child(id_map, children)?,
            composition: CompositionMode::try_from_str(params[0].enum_value()).unwrap(),
            scaling: (&params[1]).into(),
            seed: (&params[2]).into(),
            sampling: ParameterSamplingMode::try_from_str(params[3].enum_value()).unwrap(),
        }))
    }
}

impl SpecificMetaNodeKind for MetaSimilarity {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Similarity",
        hover_text: "Similarity transformation (scale, rotate, translate) of one or more instances.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::Instances;
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        single_child_port_kind(MetaChildPortKind::Instances);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaEnumParam::new(
                LabelAndHoverText {
                    label: "Composition",
                    hover_text: "Whether to apply the similarity transform after ('Post') or before ('Pre') the transforms of the input instances.",
                },
                 EnumParamVariants::from_iter(["Post", "Pre"]),
                "Post",
            )
        );
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Scale",
                    hover_text: "Uniform scale factor.",
                },
                1.0,
            )
            .with_min_value(1e-3)
            .with_speed(0.005),
        );
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Tilt angle",
                    hover_text: "Angle away from the y-axis, in degrees.",
                },
                0.0,
            )
            .with_speed(0.03),
        );
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Turn angle",
                    hover_text: "Angle from the x-axis in the xz-plane, in degrees.",
                },
                0.0,
            )
            .with_speed(0.03),
        );
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Roll angle",
                    hover_text: "Additional roll angle around the final rotated axis, in degrees.",
                },
                0.0,
            )
            .with_speed(0.03),
        );
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Translation x",
                    hover_text: "Translation distance along the x-axis, in voxels.",
                },
                0.0,
            )
            .with_speed(0.05),
        );
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Translation y",
                    hover_text: "Translation distance along the y-axis, in voxels.",
                },
                0.0,
            )
            .with_speed(0.05),
        );
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Translation z",
                    hover_text: "Translation distance along the z-axis, in voxels.",
                },
                0.0,
            )
            .with_speed(0.05),
        );
        params.push(MetaUIntParam::new(
            LabelAndHoverText {
                label: "Seed",
                hover_text: "Seed for generating randomized similarity transforms.",
            },
            0,
        ));
        params.push(
            MetaEnumParam::new(
                LabelAndHoverText {
                    label: "Sampling",
                    hover_text: "How to sample parameters from distributions when there are multiple instances.",
                },
                 EnumParamVariants::from_iter(["Only once", "Per instance"]),
                "Only once",
            )
        );
        params
    }

    fn build<A: Allocator>(
        id_map: &NoHashMap<MetaNodeID, MetaSDFNodeID, A>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 10);
        Some(MetaSDFNode::Similarity(MetaSimilarity {
            child_id: unary_child(id_map, children)?,
            composition: CompositionMode::try_from_str(params[0].enum_value()).unwrap(),
            scale: (&params[1]).into(),
            tilt_angle: (&params[2]).into(),
            turn_angle: (&params[3]).into(),
            roll_angle: (&params[4]).into(),
            translation_x: (&params[5]).into(),
            translation_y: (&params[6]).into(),
            translation_z: (&params[7]).into(),
            seed: (&params[8]).into(),
            sampling: ParameterSamplingMode::try_from_str(params[9].enum_value()).unwrap(),
        }))
    }
}

impl SpecificMetaNodeKind for MetaStratifiedGridTransforms {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Stratified grid transforms",
        hover_text: "Translation of instances from the center of a grid to grid points picked by stratified sampling.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::Instances;
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        single_child_port_kind(MetaChildPortKind::Instances);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(MetaDistributedParam::new_fixed_constant_discrete_value(
            LabelAndHoverText {
                label: "Size x",
                hover_text: "Number of grid cells along the x-axis.",
            },
            1,
        ));
        params.push(MetaDistributedParam::new_fixed_constant_discrete_value(
            LabelAndHoverText {
                label: "Size y",
                hover_text: "Number of grid cells along the y-axis.",
            },
            1,
        ));
        params.push(MetaDistributedParam::new_fixed_constant_discrete_value(
            LabelAndHoverText {
                label: "Size z",
                hover_text: "Number of grid cells along the z-axis.",
            },
            1,
        ));
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Cell extent x",
                    hover_text: "Extent of a grid cell along the x-axis, in voxels.",
                },
                60.0,
            )
            .with_min_value(0.0),
        );
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Cell extent y",
                    hover_text: "Extent of a grid cell along the y-axis, in voxels.",
                },
                60.0,
            )
            .with_min_value(0.0),
        );
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Cell extent z",
                    hover_text: "Extent of a grid cell along the z-axis, in voxels.",
                },
                60.0,
            )
            .with_min_value(0.0),
        );
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Jitter fraction",
                    hover_text: "Fraction of a grid cell to randomly displace the points.",
                },
                0.0,
            )
            .with_min_value(0.0)
            .with_max_value(1.0)
            .with_speed(0.001),
        );
        params.push(
            MetaUIntParam::new(
                LabelAndHoverText {
                    label: "Seed",
                    hover_text: "Seed for random jittering as well as generating randomized parameter values.",
                },
                0,
            )
        );
        params
    }

    fn build<A: Allocator>(
        id_map: &NoHashMap<MetaNodeID, MetaSDFNodeID, A>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 8);
        Some(MetaSDFNode::StratifiedGridTransforms(
            MetaStratifiedGridTransforms {
                child_id: unary_child(id_map, children)?,
                shape_x: (&params[0]).into(),
                shape_y: (&params[1]).into(),
                shape_z: (&params[2]).into(),
                cell_extent_x: (&params[3]).into(),
                cell_extent_y: (&params[4]).into(),
                cell_extent_z: (&params[5]).into(),
                jitter_fraction: (&params[6]).into(),
                seed: (&params[7]).into(),
            },
        ))
    }
}

impl SpecificMetaNodeKind for MetaSphereSurfaceTransforms {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Sphere surface transforms",
        hover_text: "Translation of instances from the center to the surface of a sphere, with optional rotations from the y-axis to the radial direction.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::Instances;
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        single_child_port_kind(MetaChildPortKind::Instances);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Radius",
                    hover_text: "Radius of the sphere, in voxels.",
                },
                30.0,
            )
            .with_min_value(0.0),
        );
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Jitter fraction",
                    hover_text: "Fraction of the regular point spacing to randomly displace the points.",
                },
                0.0,
            )
            .with_min_value(0.0)
            .with_max_value(1.0)
            .with_speed(0.001)
        );
        params.push(
            MetaEnumParam::new(
                LabelAndHoverText {
                    label: "Rotation",
                    hover_text: "Whether to include rotations from the y-axes to the outward or inward radial direction.",
                },
                EnumParamVariants::from_iter(["Identity", "Radial (outwards)", "Radial (inwards)"]),
                "Identity",
            )
        );
        params.push(
            MetaUIntParam::new(
                LabelAndHoverText {
                    label: "Seed",
                    hover_text: "Seed for random jittering as well as generating randomized parameter values.",
                },
                0,
            )
        );
        params
    }

    fn build<A: Allocator>(
        id_map: &NoHashMap<MetaNodeID, MetaSDFNodeID, A>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 4);
        Some(MetaSDFNode::SphereSurfaceTransforms(
            MetaSphereSurfaceTransforms {
                child_id: unary_child(id_map, children)?,
                radius: (&params[0]).into(),
                jitter_fraction: (&params[1]).into(),
                rotation: SphereSurfaceRotation::try_from_str(params[2].enum_value()).unwrap(),
                seed: (&params[3]).into(),
            },
        ))
    }
}

impl SpecificMetaNodeKind for MetaClosestTranslationToSurface {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Closest translation to surface",
        hover_text: "Translation of the instances in the second input to the closest points on the surface of the SDF in the first input.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::Instances;
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        two_child_port_kinds(MetaChildPortKind::SingleSDF, MetaChildPortKind::Instances);

    fn params() -> MetaNodeParams {
        MetaNodeParams::new()
    }

    fn build<A: Allocator>(
        id_map: &NoHashMap<MetaNodeID, MetaSDFNodeID, A>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 0);
        let (surface_sdf_id, subject_id) = binary_children(id_map, children)?;
        Some(MetaSDFNode::ClosestTranslationToSurface(
            MetaClosestTranslationToSurface {
                surface_sdf_id,
                subject_id,
            },
        ))
    }
}

impl SpecificMetaNodeKind for MetaRayTranslationToSurface {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Ray translation to surface",
        hover_text: "Translation of the instances in the second input to the intersection of their y-axes with the surface of the SDF in the first input.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::Instances;
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        two_child_port_kinds(MetaChildPortKind::SingleSDF, MetaChildPortKind::Instances);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(
            MetaEnumParam::new(
                LabelAndHoverText {
                    label: "Anchor",
                    hover_text: "The anchor (origin or shape boundary) that should be translated to the surface.",
                },
                 EnumParamVariants::from_iter(["Origin", "Shape boundary at origin"]),
                "Origin",
            )
        );
        params
    }

    fn build<A: Allocator>(
        id_map: &NoHashMap<MetaNodeID, MetaSDFNodeID, A>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 1);
        let (surface_sdf_id, subject_id) = binary_children(id_map, children)?;
        Some(MetaSDFNode::RayTranslationToSurface(
            MetaRayTranslationToSurface {
                surface_sdf_id,
                subject_id,
                anchor: RayTranslationAnchor::try_from_str(params[0].enum_value()).unwrap(),
            },
        ))
    }
}

impl SpecificMetaNodeKind for MetaRotationToGradient {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Rotation to gradient",
        hover_text: "Rotation of the instances in the second input to make their y-axis align with the gradient of the SDF in the first input.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::Instances;
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        two_child_port_kinds(MetaChildPortKind::SingleSDF, MetaChildPortKind::Instances);

    fn params() -> MetaNodeParams {
        MetaNodeParams::new()
    }

    fn build<A: Allocator>(
        id_map: &NoHashMap<MetaNodeID, MetaSDFNodeID, A>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 0);
        let (gradient_sdf_id, subject_id) = binary_children(id_map, children)?;
        Some(MetaSDFNode::RotationToGradient(MetaRotationToGradient {
            gradient_sdf_id,
            subject_id,
        }))
    }
}

impl SpecificMetaNodeKind for MetaStochasticSelection {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Stochastic selection",
        hover_text: "Random selection of SDFs or instances from a group.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SameAsInput { slot: 0 };
    const CHILD_PORT_KINDS: MetaChildPortKinds = single_child_port_kind(MetaChildPortKind::Any);

    fn params() -> MetaNodeParams {
        let mut params = MetaNodeParams::new();
        params.push(MetaUIntParam::new(
            LabelAndHoverText {
                label: "Min count",
                hover_text: "Minimum number of items to select initially.",
            },
            1,
        ));
        params.push(MetaUIntParam::new(
            LabelAndHoverText {
                label: "Max count",
                hover_text: "Maximum number of items to select initially.",
            },
            1,
        ));
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
        );
        params.push(MetaUIntParam::new(
            LabelAndHoverText {
                label: "Seed",
                hover_text: "Seed for random selection.",
            },
            0,
        ));
        params
    }

    fn build<A: Allocator>(
        id_map: &NoHashMap<MetaNodeID, MetaSDFNodeID, A>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 4);
        Some(MetaSDFNode::StochasticSelection(MetaStochasticSelection {
            child_id: unary_child(id_map, children)?,
            min_pick_count: (&params[0]).into(),
            max_pick_count: (&params[1]).into(),
            pick_probability: (&params[2]).into(),
            seed: (&params[3]).into(),
        }))
    }
}

impl SpecificMetaNodeKind for MetaSDFInstantiation {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "SDF instantiation",
        hover_text: "Instantiation of the input instances into SDFs using their shapes and transforms. Instances with no shape produce no SDFs.",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SDFGroup;
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        single_child_port_kind(MetaChildPortKind::Instances);

    fn params() -> MetaNodeParams {
        MetaNodeParams::new()
    }

    fn build<A: Allocator>(
        id_map: &NoHashMap<MetaNodeID, MetaSDFNodeID, A>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 0);
        Some(MetaSDFNode::SDFInstantiation(MetaSDFInstantiation {
            child_id: unary_child(id_map, children)?,
        }))
    }
}

impl SpecificMetaNodeKind for MetaTransformApplication {
    const LABEL: LabelAndHoverText = LabelAndHoverText {
        label: "Transform application",
        hover_text: "Application of the transforms of the instances in the second input to the SDFs in the first input (yields all combinations).",
    };
    const PARENT_PORT_KIND: MetaParentPortKind = MetaParentPortKind::SDFGroup;
    const CHILD_PORT_KINDS: MetaChildPortKinds =
        two_child_port_kinds(MetaChildPortKind::SDFGroup, MetaChildPortKind::Instances);

    fn params() -> MetaNodeParams {
        MetaNodeParams::new()
    }

    fn build<A: Allocator>(
        id_map: &NoHashMap<MetaNodeID, MetaSDFNodeID, A>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 0);
        let (sdf_id, instance_id) = binary_children(id_map, children)?;
        Some(MetaSDFNode::TransformApplication(
            MetaTransformApplication {
                sdf_id,
                instance_id,
            },
        ))
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
            MetaDistributedParam::new_fixed_constant_discrete_value(
                LabelAndHoverText {
                    label: "Octaves",
                    hover_text: "Number of noise octaves (patterns of increasing frequency) to combine.",
                },
                1,
            )
        );
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Frequency",
                    hover_text: "Spatial frequency of the noise pattern in the first octave, in inverse voxels.",
                },
                0.02,
            )
            .with_min_value(0.0)
            .with_max_value(1.0)
            .with_speed(0.0002)
        );
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Lacunarity",
                    hover_text: "Noise frequency multiplier between successive octaves.",
                },
                2.0,
            )
            .with_min_value(1.0)
            .with_max_value(10.0)
            .with_speed(0.001),
        );
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Persistence",
                    hover_text: "Noise amplitude multiplier between successive octaves.",
                },
                0.5,
            )
            .with_min_value(0.0)
            .with_max_value(1.0)
            .with_speed(0.001),
        );
        params.push(
            MetaDistributedParam::new_fixed_constant_continuous_value(
                LabelAndHoverText {
                    label: "Amplitude",
                    hover_text: "Noise amplitude (max displacement) in the first octave, in voxels.",
                },
                5.0,
            )
            .with_min_value(0.0)
            .with_speed(0.05)
        );
        params.push(MetaUIntParam::new(
            LabelAndHoverText {
                label: "Seed",
                hover_text: "Seed for generating noise and randomized parameter values.",
            },
            0,
        ));
        params.push(
            MetaEnumParam::new(
                LabelAndHoverText {
                    label: "Sampling",
                    hover_text: "How to sample parameters from distributions when there are multiple SDFs.",
                },
                 EnumParamVariants::from_iter(["Only once", "Per SDF"]),
                "Only once",
            )
        );
        params
    }

    fn build<A: Allocator>(
        id_map: &NoHashMap<MetaNodeID, MetaSDFNodeID, A>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 7);
        Some(MetaSDFNode::MultifractalNoiseSDFModifier(
            MetaMultifractalNoiseSDFModifier {
                child_id: unary_child(id_map, children)?,
                octaves: (&params[0]).into(),
                frequency: (&params[1]).into(),
                lacunarity: (&params[2]).into(),
                persistence: (&params[3]).into(),
                amplitude: (&params[4]).into(),
                seed: (&params[5]).into(),
                sampling: ParameterSamplingMode::try_from_str(params[6].enum_value()).unwrap(),
            },
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
            .with_min_value(0.0),
        );
        params
    }

    fn build<A: Allocator>(
        id_map: &NoHashMap<MetaNodeID, MetaSDFNodeID, A>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 1);
        let (child_1_id, child_2_id) = binary_children(id_map, children)?;
        Some(MetaSDFNode::SDFUnion(MetaSDFUnion {
            child_1_id,
            child_2_id,
            smoothness: (&params[0]).into(),
        }))
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
            .with_min_value(0.0),
        );
        params
    }

    fn build<A: Allocator>(
        id_map: &NoHashMap<MetaNodeID, MetaSDFNodeID, A>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 1);
        let (child_1_id, child_2_id) = binary_children(id_map, children)?;
        Some(MetaSDFNode::SDFSubtraction(MetaSDFSubtraction {
            child_1_id,
            child_2_id,
            smoothness: (&params[0]).into(),
        }))
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
            .with_min_value(0.0),
        );
        params
    }

    fn build<A: Allocator>(
        id_map: &NoHashMap<MetaNodeID, MetaSDFNodeID, A>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 1);
        let (child_1_id, child_2_id) = binary_children(id_map, children)?;
        Some(MetaSDFNode::SDFIntersection(MetaSDFIntersection {
            child_1_id,
            child_2_id,
            smoothness: (&params[0]).into(),
        }))
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
        );
        params
    }

    fn build<A: Allocator>(
        id_map: &NoHashMap<MetaNodeID, MetaSDFNodeID, A>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        assert_eq!(params.len(), 1);
        Some(MetaSDFNode::SDFGroupUnion(MetaSDFGroupUnion {
            child_id: unary_child(id_map, children)?,
            smoothness: (&params[0]).into(),
        }))
    }
}

impl MetaNodeKind {
    pub const fn all_non_root() -> [Self; 21] {
        [
            Self::Points,
            Self::Spheres,
            Self::Capsules,
            Self::Boxes,
            Self::Translation,
            Self::Rotation,
            Self::Scaling,
            Self::Similarity,
            Self::StratifiedGridTransforms,
            Self::SphereSurfaceTransforms,
            Self::ClosestTranslationToSurface,
            Self::RayTranslationToSurface,
            Self::RotationToGradient,
            Self::StochasticSelection,
            Self::SDFInstantiation,
            Self::TransformApplication,
            Self::MultifractalNoiseSDFModifier,
            Self::SDFUnion,
            Self::SDFSubtraction,
            Self::SDFIntersection,
            Self::SDFGroupUnion,
        ]
    }

    pub fn is_output(&self) -> bool {
        *self == Self::Output
    }

    pub const fn group(&self) -> MetaNodeKindGroup {
        match self {
            Self::Output => MetaNodeKindGroup::Root,
            Self::Points | Self::Spheres | Self::Capsules | Self::Boxes => {
                MetaNodeKindGroup::InstancePrimitives
            }
            Self::Translation | Self::Rotation | Self::Scaling | Self::Similarity => {
                MetaNodeKindGroup::BasicInstanceTransforms
            }
            Self::StratifiedGridTransforms | Self::SphereSurfaceTransforms => {
                MetaNodeKindGroup::StructuredInstanceTransforms
            }
            Self::ClosestTranslationToSurface
            | Self::RayTranslationToSurface
            | Self::RotationToGradient => MetaNodeKindGroup::SDFBasedInstanceTransforms,
            Self::StochasticSelection => MetaNodeKindGroup::Filtering,
            Self::SDFInstantiation | Self::TransformApplication => {
                MetaNodeKindGroup::SDFFromInstances
            }
            Self::MultifractalNoiseSDFModifier => MetaNodeKindGroup::SDFModifiers,
            Self::SDFUnion | Self::SDFSubtraction | Self::SDFIntersection | Self::SDFGroupUnion => {
                MetaNodeKindGroup::SDFCombination
            }
        }
    }

    pub const fn label(&self) -> LabelAndHoverText {
        match self {
            Self::Output => LabelAndHoverText::label_only("Output"),
            Self::Points => MetaPoints::LABEL,
            Self::Spheres => MetaSpheres::LABEL,
            Self::Capsules => MetaCapsules::LABEL,
            Self::Boxes => MetaBoxes::LABEL,
            Self::Translation => MetaTranslation::LABEL,
            Self::Rotation => MetaRotation::LABEL,
            Self::Scaling => MetaScaling::LABEL,
            Self::Similarity => MetaSimilarity::LABEL,
            Self::StratifiedGridTransforms => MetaStratifiedGridTransforms::LABEL,
            Self::SphereSurfaceTransforms => MetaSphereSurfaceTransforms::LABEL,
            Self::ClosestTranslationToSurface => MetaClosestTranslationToSurface::LABEL,
            Self::RayTranslationToSurface => MetaRayTranslationToSurface::LABEL,
            Self::RotationToGradient => MetaRotationToGradient::LABEL,
            Self::StochasticSelection => MetaStochasticSelection::LABEL,
            Self::SDFInstantiation => MetaSDFInstantiation::LABEL,
            Self::TransformApplication => MetaTransformApplication::LABEL,
            Self::MultifractalNoiseSDFModifier => MetaMultifractalNoiseSDFModifier::LABEL,
            Self::SDFUnion => MetaSDFUnion::LABEL,
            Self::SDFSubtraction => MetaSDFSubtraction::LABEL,
            Self::SDFIntersection => MetaSDFIntersection::LABEL,
            Self::SDFGroupUnion => MetaSDFGroupUnion::LABEL,
        }
    }

    pub const fn parent_port_kind(&self) -> MetaParentPortKind {
        match self {
            Self::Output => MetaParentPortKind::SingleSDF,
            Self::Points => MetaPoints::PARENT_PORT_KIND,
            Self::Spheres => MetaSpheres::PARENT_PORT_KIND,
            Self::Capsules => MetaCapsules::PARENT_PORT_KIND,
            Self::Boxes => MetaBoxes::PARENT_PORT_KIND,
            Self::Translation => MetaTranslation::PARENT_PORT_KIND,
            Self::Rotation => MetaRotation::PARENT_PORT_KIND,
            Self::Scaling => MetaScaling::PARENT_PORT_KIND,
            Self::Similarity => MetaSimilarity::PARENT_PORT_KIND,
            Self::StratifiedGridTransforms => MetaStratifiedGridTransforms::PARENT_PORT_KIND,
            Self::SphereSurfaceTransforms => MetaSphereSurfaceTransforms::PARENT_PORT_KIND,
            Self::ClosestTranslationToSurface => MetaClosestTranslationToSurface::PARENT_PORT_KIND,
            Self::RayTranslationToSurface => MetaRayTranslationToSurface::PARENT_PORT_KIND,
            Self::RotationToGradient => MetaRotationToGradient::PARENT_PORT_KIND,
            Self::StochasticSelection => MetaStochasticSelection::PARENT_PORT_KIND,
            Self::SDFInstantiation => MetaSDFInstantiation::PARENT_PORT_KIND,
            Self::TransformApplication => MetaTransformApplication::PARENT_PORT_KIND,
            Self::MultifractalNoiseSDFModifier => {
                MetaMultifractalNoiseSDFModifier::PARENT_PORT_KIND
            }
            Self::SDFUnion => MetaSDFUnion::PARENT_PORT_KIND,
            Self::SDFSubtraction => MetaSDFSubtraction::PARENT_PORT_KIND,
            Self::SDFIntersection => MetaSDFIntersection::PARENT_PORT_KIND,
            Self::SDFGroupUnion => MetaSDFGroupUnion::PARENT_PORT_KIND,
        }
    }

    const fn raw_child_port_kinds(&self) -> MetaChildPortKinds {
        match self {
            Self::Output => single_child_port_kind(MetaChildPortKind::SingleSDF),
            Self::Points => MetaPoints::CHILD_PORT_KINDS,
            Self::Spheres => MetaSpheres::CHILD_PORT_KINDS,
            Self::Capsules => MetaCapsules::CHILD_PORT_KINDS,
            Self::Boxes => MetaBoxes::CHILD_PORT_KINDS,
            Self::Translation => MetaTranslation::CHILD_PORT_KINDS,
            Self::Rotation => MetaRotation::CHILD_PORT_KINDS,
            Self::Scaling => MetaScaling::CHILD_PORT_KINDS,
            Self::Similarity => MetaSimilarity::CHILD_PORT_KINDS,
            Self::StratifiedGridTransforms => MetaStratifiedGridTransforms::CHILD_PORT_KINDS,
            Self::SphereSurfaceTransforms => MetaSphereSurfaceTransforms::CHILD_PORT_KINDS,
            Self::ClosestTranslationToSurface => MetaClosestTranslationToSurface::CHILD_PORT_KINDS,
            Self::RayTranslationToSurface => MetaRayTranslationToSurface::CHILD_PORT_KINDS,
            Self::RotationToGradient => MetaRotationToGradient::CHILD_PORT_KINDS,
            Self::StochasticSelection => MetaStochasticSelection::CHILD_PORT_KINDS,
            Self::SDFInstantiation => MetaSDFInstantiation::CHILD_PORT_KINDS,
            Self::TransformApplication => MetaTransformApplication::CHILD_PORT_KINDS,
            Self::MultifractalNoiseSDFModifier => {
                MetaMultifractalNoiseSDFModifier::CHILD_PORT_KINDS
            }
            Self::SDFUnion => MetaSDFUnion::CHILD_PORT_KINDS,
            Self::SDFSubtraction => MetaSDFSubtraction::CHILD_PORT_KINDS,
            Self::SDFIntersection => MetaSDFIntersection::CHILD_PORT_KINDS,
            Self::SDFGroupUnion => MetaSDFGroupUnion::CHILD_PORT_KINDS,
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
            Self::Points => MetaPoints::params(),
            Self::Spheres => MetaSpheres::params(),
            Self::Capsules => MetaCapsules::params(),
            Self::Boxes => MetaBoxes::params(),
            Self::Translation => MetaTranslation::params(),
            Self::Rotation => MetaRotation::params(),
            Self::Scaling => MetaScaling::params(),
            Self::Similarity => MetaSimilarity::params(),
            Self::StratifiedGridTransforms => MetaStratifiedGridTransforms::params(),
            Self::SphereSurfaceTransforms => MetaSphereSurfaceTransforms::params(),
            Self::ClosestTranslationToSurface => MetaClosestTranslationToSurface::params(),
            Self::RayTranslationToSurface => MetaRayTranslationToSurface::params(),
            Self::RotationToGradient => MetaRotationToGradient::params(),
            Self::StochasticSelection => MetaStochasticSelection::params(),
            Self::SDFInstantiation => MetaSDFInstantiation::params(),
            Self::TransformApplication => MetaTransformApplication::params(),
            Self::MultifractalNoiseSDFModifier => MetaMultifractalNoiseSDFModifier::params(),
            Self::SDFUnion => MetaSDFUnion::params(),
            Self::SDFSubtraction => MetaSDFSubtraction::params(),
            Self::SDFIntersection => MetaSDFIntersection::params(),
            Self::SDFGroupUnion => MetaSDFGroupUnion::params(),
        }
    }

    pub fn build_sdf_generator_node<A: Allocator>(
        &self,
        id_map: &NoHashMap<MetaNodeID, MetaSDFNodeID, A>,
        children: &[Option<MetaNodeLink>],
        params: &[MetaNodeParam],
    ) -> Option<MetaSDFNode> {
        match self {
            Self::Output => None,
            Self::Points => MetaPoints::build(id_map, children, params),
            Self::Spheres => MetaSpheres::build(id_map, children, params),
            Self::Capsules => MetaCapsules::build(id_map, children, params),
            Self::Boxes => MetaBoxes::build(id_map, children, params),
            Self::Translation => MetaTranslation::build(id_map, children, params),
            Self::Rotation => MetaRotation::build(id_map, children, params),
            Self::Scaling => MetaScaling::build(id_map, children, params),
            Self::Similarity => MetaSimilarity::build(id_map, children, params),
            Self::StratifiedGridTransforms => {
                MetaStratifiedGridTransforms::build(id_map, children, params)
            }
            Self::SphereSurfaceTransforms => {
                MetaSphereSurfaceTransforms::build(id_map, children, params)
            }
            Self::ClosestTranslationToSurface => {
                MetaClosestTranslationToSurface::build(id_map, children, params)
            }
            Self::RayTranslationToSurface => {
                MetaRayTranslationToSurface::build(id_map, children, params)
            }
            Self::RotationToGradient => MetaRotationToGradient::build(id_map, children, params),
            Self::StochasticSelection => MetaStochasticSelection::build(id_map, children, params),
            Self::SDFInstantiation => MetaSDFInstantiation::build(id_map, children, params),
            Self::TransformApplication => MetaTransformApplication::build(id_map, children, params),
            Self::MultifractalNoiseSDFModifier => {
                MetaMultifractalNoiseSDFModifier::build(id_map, children, params)
            }
            Self::SDFUnion => MetaSDFUnion::build(id_map, children, params),
            Self::SDFSubtraction => MetaSDFSubtraction::build(id_map, children, params),
            Self::SDFIntersection => MetaSDFIntersection::build(id_map, children, params),
            Self::SDFGroupUnion => MetaSDFGroupUnion::build(id_map, children, params),
        }
    }
}

impl MetaNodeKindGroup {
    pub const fn all_non_root() -> [Self; 8] {
        [
            Self::InstancePrimitives,
            Self::BasicInstanceTransforms,
            Self::StructuredInstanceTransforms,
            Self::SDFBasedInstanceTransforms,
            Self::Filtering,
            Self::SDFFromInstances,
            Self::SDFModifiers,
            Self::SDFCombination,
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
    (
        (&output_node_params[0]).into(),
        (&output_node_params[1]).into(),
    )
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
        .with_speed(0.01),
    );
    params.push(MetaUIntParam::new(
        LabelAndHoverText {
            label: "Seed",
            hover_text: "Global seed offset added to the seed of all nodes.",
        },
        0,
    ));
    params
}

fn unary_child<A: Allocator>(
    id_map: &NoHashMap<MetaNodeID, MetaSDFNodeID, A>,
    children: &[Option<MetaNodeLink>],
) -> Option<MetaSDFNodeID> {
    assert_eq!(children.len(), 1);
    children
        .first()?
        .map(|attachment| id_map[&attachment.to_node])
}

fn binary_children<A: Allocator>(
    id_map: &NoHashMap<MetaNodeID, MetaSDFNodeID, A>,
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
