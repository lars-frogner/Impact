use crate::editor::UIntParam;

use super::{FloatParam, NodeID, NodeParam, PortConfig};
use impact::impact_containers::HashMap;
use impact_dev_ui::option_panels::LabelAndHoverText;
use impact_voxel::generation::{
    BoxSDFGenerator, GradientNoiseSDFGenerator, MultifractalNoiseSDFModifier,
    MultiscaleSphereSDFModifier, SDFGeneratorNode, SDFIntersection, SDFNodeID, SDFRotation,
    SDFScaling, SDFSubtraction, SDFTranslation, SDFUnion, SphereSDFGenerator,
};

trait SpecificNodeKind {
    const LABEL: &'static str;
    const PORT_CONFIG: PortConfig;

    fn default_params() -> Vec<NodeParam>;

    fn build(
        id_map: &HashMap<NodeID, SDFNodeID>,
        children: &[Option<NodeID>],
        params: &[NodeParam],
    ) -> Option<SDFGeneratorNode>;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NodeKind {
    Output,
    #[default]
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

pub const DEFAULT_VOXEL_EXTENT: f32 = 0.25;
pub const MIN_VOXEL_EXTENT: f32 = 0.005;

impl SpecificNodeKind for BoxSDFGenerator {
    const LABEL: &'static str = "Box";
    const PORT_CONFIG: PortConfig = PortConfig::leaf();

    fn default_params() -> Vec<NodeParam> {
        vec![
            NodeParam::Float(
                FloatParam::new(LabelAndHoverText::label_only("Extent x"), 14.0)
                    .with_min_value(0.0),
            ),
            NodeParam::Float(
                FloatParam::new(LabelAndHoverText::label_only("Extent y"), 14.0)
                    .with_min_value(0.0),
            ),
            NodeParam::Float(
                FloatParam::new(LabelAndHoverText::label_only("Extent z"), 14.0)
                    .with_min_value(0.0),
            ),
        ]
    }

    fn build(
        _id_map: &HashMap<NodeID, SDFNodeID>,
        _children: &[Option<NodeID>],
        params: &[NodeParam],
    ) -> Option<SDFGeneratorNode> {
        assert_eq!(params.len(), 3);
        let extents = [params[0].float(), params[1].float(), params[2].float()];
        Some(SDFGeneratorNode::Box(BoxSDFGenerator::new(extents)))
    }
}

impl SpecificNodeKind for SphereSDFGenerator {
    const LABEL: &'static str = "Sphere";
    const PORT_CONFIG: PortConfig = PortConfig::leaf();

    fn default_params() -> Vec<NodeParam> {
        vec![NodeParam::Float(
            FloatParam::new(LabelAndHoverText::label_only("Radius"), 7.0).with_min_value(0.0),
        )]
    }

    fn build(
        _id_map: &HashMap<NodeID, SDFNodeID>,
        _children: &[Option<NodeID>],
        params: &[NodeParam],
    ) -> Option<SDFGeneratorNode> {
        assert_eq!(params.len(), 1);
        let radius = params[0].float();
        Some(SDFGeneratorNode::Sphere(SphereSDFGenerator::new(radius)))
    }
}

impl SpecificNodeKind for GradientNoiseSDFGenerator {
    const LABEL: &'static str = "Gradient noise";
    const PORT_CONFIG: PortConfig = PortConfig::leaf();

    fn default_params() -> Vec<NodeParam> {
        vec![
            NodeParam::Float(
                FloatParam::new(LabelAndHoverText::label_only("Extent x"), 14.0)
                    .with_min_value(0.0),
            ),
            NodeParam::Float(
                FloatParam::new(LabelAndHoverText::label_only("Extent y"), 14.0)
                    .with_min_value(0.0),
            ),
            NodeParam::Float(
                FloatParam::new(LabelAndHoverText::label_only("Extent z"), 14.0)
                    .with_min_value(0.0),
            ),
            NodeParam::Float(FloatParam::new(
                LabelAndHoverText::label_only("Frequency"),
                1.0,
            )),
            NodeParam::Float(FloatParam::new(
                LabelAndHoverText::label_only("Threshold"),
                1.0,
            )),
            NodeParam::UInt(UIntParam::new(LabelAndHoverText::label_only("Seed"), 0)),
        ]
    }

    fn build(
        _id_map: &HashMap<NodeID, SDFNodeID>,
        _children: &[Option<NodeID>],
        params: &[NodeParam],
    ) -> Option<SDFGeneratorNode> {
        assert_eq!(params.len(), 6);
        let extents = [params[0].float(), params[1].float(), params[2].float()];
        let noise_frequency = params[3].float();
        let noise_threshold = params[4].float();
        let seed = params[5].uint();
        Some(SDFGeneratorNode::GradientNoise(
            GradientNoiseSDFGenerator::new(extents, noise_frequency, noise_threshold, seed),
        ))
    }
}

impl SpecificNodeKind for SDFTranslation {
    const LABEL: &'static str = "Translation";
    const PORT_CONFIG: PortConfig = PortConfig::unary();

    fn default_params() -> Vec<NodeParam> {
        vec![
            NodeParam::Float(FloatParam::new(LabelAndHoverText::label_only("In x"), 0.0)),
            NodeParam::Float(FloatParam::new(LabelAndHoverText::label_only("In y"), 0.0)),
            NodeParam::Float(FloatParam::new(LabelAndHoverText::label_only("In z"), 0.0)),
        ]
    }

    fn build(
        id_map: &HashMap<NodeID, SDFNodeID>,
        children: &[Option<NodeID>],
        params: &[NodeParam],
    ) -> Option<SDFGeneratorNode> {
        assert_eq!(params.len(), 3);
        let child_id = unary_child(id_map, children)?;
        let translation = [params[0].float(), params[1].float(), params[2].float()].into();
        Some(SDFGeneratorNode::Translation(SDFTranslation {
            child_id,
            translation,
        }))
    }
}

impl SpecificNodeKind for SDFRotation {
    const LABEL: &'static str = "Rotation";
    const PORT_CONFIG: PortConfig = PortConfig::unary();

    fn default_params() -> Vec<NodeParam> {
        vec![
            NodeParam::Float(FloatParam::new(
                LabelAndHoverText::label_only("Axis x"),
                0.0,
            )),
            NodeParam::Float(FloatParam::new(
                LabelAndHoverText::label_only("Axis y"),
                0.0,
            )),
            NodeParam::Float(FloatParam::new(
                LabelAndHoverText::label_only("Axis z"),
                1.0,
            )),
            NodeParam::Float(FloatParam::new(LabelAndHoverText::label_only("Angle"), 0.0)),
        ]
    }

    fn build(
        id_map: &HashMap<NodeID, SDFNodeID>,
        children: &[Option<NodeID>],
        params: &[NodeParam],
    ) -> Option<SDFGeneratorNode> {
        assert_eq!(params.len(), 4);
        let child_id = unary_child(id_map, children)?;
        let axis = [params[0].float(), params[1].float(), params[2].float()].into();
        let angle = params[3].float();
        Some(SDFGeneratorNode::Rotation(SDFRotation::from_axis_angle(
            child_id, axis, angle,
        )))
    }
}

impl SpecificNodeKind for SDFScaling {
    const LABEL: &'static str = "Scaling";
    const PORT_CONFIG: PortConfig = PortConfig::unary();

    fn default_params() -> Vec<NodeParam> {
        vec![NodeParam::Float(
            FloatParam::new(LabelAndHoverText::label_only("Factor"), 1.0).with_min_value(0.0),
        )]
    }

    fn build(
        id_map: &HashMap<NodeID, SDFNodeID>,
        children: &[Option<NodeID>],
        params: &[NodeParam],
    ) -> Option<SDFGeneratorNode> {
        assert_eq!(params.len(), 1);
        let child_id = unary_child(id_map, children)?;
        let scaling = params[0].float();
        Some(SDFGeneratorNode::Scaling(SDFScaling::new(
            child_id, scaling,
        )))
    }
}

impl SpecificNodeKind for MultifractalNoiseSDFModifier {
    const LABEL: &'static str = "Multifractal noise";
    const PORT_CONFIG: PortConfig = PortConfig::unary();

    fn default_params() -> Vec<NodeParam> {
        vec![
            NodeParam::UInt(UIntParam::new(LabelAndHoverText::label_only("Octaves"), 0)),
            NodeParam::Float(FloatParam::new(
                LabelAndHoverText::label_only("Frequency"),
                1.0,
            )),
            NodeParam::Float(FloatParam::new(
                LabelAndHoverText::label_only("Lacunarity"),
                1.0,
            )),
            NodeParam::Float(FloatParam::new(
                LabelAndHoverText::label_only("Persistence"),
                1.0,
            )),
            NodeParam::Float(FloatParam::new(
                LabelAndHoverText::label_only("Amplitude"),
                1.0,
            )),
            NodeParam::UInt(UIntParam::new(LabelAndHoverText::label_only("Seed"), 0)),
        ]
    }

    fn build(
        id_map: &HashMap<NodeID, SDFNodeID>,
        children: &[Option<NodeID>],
        params: &[NodeParam],
    ) -> Option<SDFGeneratorNode> {
        assert_eq!(params.len(), 6);
        let child_id = unary_child(id_map, children)?;
        let octaves = params[0].uint();
        let frequency = params[1].float();
        let lacunarity = params[2].float();
        let persistence = params[3].float();
        let amplitude = params[4].float();
        let seed = params[5].uint();
        Some(SDFGeneratorNode::MultifractalNoise(
            MultifractalNoiseSDFModifier::new(
                child_id,
                octaves,
                frequency,
                lacunarity,
                persistence,
                amplitude,
                seed,
            ),
        ))
    }
}

impl SpecificNodeKind for MultiscaleSphereSDFModifier {
    const LABEL: &'static str = "Multiscale sphere";
    const PORT_CONFIG: PortConfig = PortConfig::unary();

    fn default_params() -> Vec<NodeParam> {
        vec![
            NodeParam::UInt(UIntParam::new(LabelAndHoverText::label_only("Octaves"), 0)),
            NodeParam::Float(FloatParam::new(
                LabelAndHoverText::label_only("Max scale"),
                1.0,
            )),
            NodeParam::Float(FloatParam::new(
                LabelAndHoverText::label_only("Persistence"),
                1.0,
            )),
            NodeParam::Float(FloatParam::new(
                LabelAndHoverText::label_only("Inflation"),
                1.0,
            )),
            NodeParam::Float(FloatParam::new(
                LabelAndHoverText::label_only("Smoothness"),
                1.0,
            )),
            NodeParam::UInt(UIntParam::new(LabelAndHoverText::label_only("Seed"), 0)),
        ]
    }

    fn build(
        id_map: &HashMap<NodeID, SDFNodeID>,
        children: &[Option<NodeID>],
        params: &[NodeParam],
    ) -> Option<SDFGeneratorNode> {
        assert_eq!(params.len(), 6);
        let child_id = unary_child(id_map, children)?;
        let octaves = params[0].uint();
        let max_scale = params[1].float();
        let persistence = params[2].float();
        let inflation = params[3].float();
        let smoothness = params[4].float();
        let seed = params[5].uint();
        Some(SDFGeneratorNode::MultiscaleSphere(
            MultiscaleSphereSDFModifier::new(
                child_id,
                octaves,
                max_scale,
                persistence,
                inflation,
                smoothness,
                seed,
            ),
        ))
    }
}

impl SpecificNodeKind for SDFUnion {
    const LABEL: &'static str = "Union";
    const PORT_CONFIG: PortConfig = PortConfig::binary();

    fn default_params() -> Vec<NodeParam> {
        vec![NodeParam::Float(
            FloatParam::new(LabelAndHoverText::label_only("Smoothness"), 1.0).with_min_value(0.0),
        )]
    }

    fn build(
        id_map: &HashMap<NodeID, SDFNodeID>,
        children: &[Option<NodeID>],
        params: &[NodeParam],
    ) -> Option<SDFGeneratorNode> {
        assert_eq!(params.len(), 1);
        let (child_1_id, child_2_id) = binary_children(id_map, children)?;
        let smoothness = params[0].float();
        Some(SDFGeneratorNode::Union(SDFUnion::new(
            child_1_id, child_2_id, smoothness,
        )))
    }
}

impl SpecificNodeKind for SDFSubtraction {
    const LABEL: &'static str = "Subtraction";
    const PORT_CONFIG: PortConfig = PortConfig::binary();

    fn default_params() -> Vec<NodeParam> {
        vec![NodeParam::Float(
            FloatParam::new(LabelAndHoverText::label_only("Smoothness"), 1.0).with_min_value(0.0),
        )]
    }

    fn build(
        id_map: &HashMap<NodeID, SDFNodeID>,
        children: &[Option<NodeID>],
        params: &[NodeParam],
    ) -> Option<SDFGeneratorNode> {
        assert_eq!(params.len(), 1);
        let (child_1_id, child_2_id) = binary_children(id_map, children)?;
        let smoothness = params[0].float();
        Some(SDFGeneratorNode::Subtraction(SDFSubtraction::new(
            child_1_id, child_2_id, smoothness,
        )))
    }
}

impl SpecificNodeKind for SDFIntersection {
    const LABEL: &'static str = "Intersection";
    const PORT_CONFIG: PortConfig = PortConfig::binary();

    fn default_params() -> Vec<NodeParam> {
        vec![NodeParam::Float(
            FloatParam::new(LabelAndHoverText::label_only("Smoothness"), 1.0).with_min_value(0.0),
        )]
    }

    fn build(
        id_map: &HashMap<NodeID, SDFNodeID>,
        children: &[Option<NodeID>],
        params: &[NodeParam],
    ) -> Option<SDFGeneratorNode> {
        assert_eq!(params.len(), 1);
        let (child_1_id, child_2_id) = binary_children(id_map, children)?;
        let smoothness = params[0].float();
        Some(SDFGeneratorNode::Intersection(SDFIntersection::new(
            child_1_id, child_2_id, smoothness,
        )))
    }
}

impl NodeKind {
    pub const fn all_non_root() -> [Self; 11] {
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
        ]
    }

    pub fn is_root(&self) -> bool {
        *self == Self::Output
    }

    pub const fn label(&self) -> &'static str {
        match self {
            Self::Output => "Output",
            Self::Box => BoxSDFGenerator::LABEL,
            Self::Sphere => SphereSDFGenerator::LABEL,
            Self::GradientNoise => GradientNoiseSDFGenerator::LABEL,
            Self::Translation => SDFTranslation::LABEL,
            Self::Rotation => SDFRotation::LABEL,
            Self::Scaling => SDFScaling::LABEL,
            Self::MultifractalNoise => MultifractalNoiseSDFModifier::LABEL,
            Self::MultiscaleSphere => MultiscaleSphereSDFModifier::LABEL,
            Self::Union => SDFUnion::LABEL,
            Self::Subtraction => SDFSubtraction::LABEL,
            Self::Intersection => SDFIntersection::LABEL,
        }
    }

    pub const fn port_config(&self) -> PortConfig {
        match self {
            Self::Output => PortConfig::root(),
            Self::Box => BoxSDFGenerator::PORT_CONFIG,
            Self::Sphere => SphereSDFGenerator::PORT_CONFIG,
            Self::GradientNoise => GradientNoiseSDFGenerator::PORT_CONFIG,
            Self::Translation => SDFTranslation::PORT_CONFIG,
            Self::Rotation => SDFRotation::PORT_CONFIG,
            Self::Scaling => SDFScaling::PORT_CONFIG,
            Self::MultifractalNoise => MultifractalNoiseSDFModifier::PORT_CONFIG,
            Self::MultiscaleSphere => MultiscaleSphereSDFModifier::PORT_CONFIG,
            Self::Union => SDFUnion::PORT_CONFIG,
            Self::Subtraction => SDFSubtraction::PORT_CONFIG,
            Self::Intersection => SDFIntersection::PORT_CONFIG,
        }
    }

    pub fn default_params(&self) -> Vec<NodeParam> {
        match self {
            Self::Output => output_node_params(),
            Self::Box => BoxSDFGenerator::default_params(),
            Self::Sphere => SphereSDFGenerator::default_params(),
            Self::GradientNoise => GradientNoiseSDFGenerator::default_params(),
            Self::Translation => SDFTranslation::default_params(),
            Self::Rotation => SDFRotation::default_params(),
            Self::Scaling => SDFScaling::default_params(),
            Self::MultifractalNoise => MultifractalNoiseSDFModifier::default_params(),
            Self::MultiscaleSphere => MultiscaleSphereSDFModifier::default_params(),
            Self::Union => SDFUnion::default_params(),
            Self::Subtraction => SDFSubtraction::default_params(),
            Self::Intersection => SDFIntersection::default_params(),
        }
    }

    pub fn build_sdf_generator_node(
        &self,
        id_map: &HashMap<NodeID, SDFNodeID>,
        children: &[Option<NodeID>],
        params: &[NodeParam],
    ) -> Option<SDFGeneratorNode> {
        match self {
            Self::Output => None,
            Self::Box => BoxSDFGenerator::build(id_map, children, params),
            Self::Sphere => SphereSDFGenerator::build(id_map, children, params),
            Self::GradientNoise => GradientNoiseSDFGenerator::build(id_map, children, params),
            Self::Translation => SDFTranslation::build(id_map, children, params),
            Self::Rotation => SDFRotation::build(id_map, children, params),
            Self::Scaling => SDFScaling::build(id_map, children, params),
            Self::MultifractalNoise => {
                MultifractalNoiseSDFModifier::build(id_map, children, params)
            }
            Self::MultiscaleSphere => MultiscaleSphereSDFModifier::build(id_map, children, params),
            Self::Union => SDFUnion::build(id_map, children, params),
            Self::Subtraction => SDFSubtraction::build(id_map, children, params),
            Self::Intersection => SDFIntersection::build(id_map, children, params),
        }
    }
}

pub fn get_voxel_extent_from_output_node(output_node_params: &[NodeParam]) -> f32 {
    output_node_params[0].float()
}

fn output_node_params() -> Vec<NodeParam> {
    vec![NodeParam::Float(
        FloatParam::new(
            LabelAndHoverText::label_only("Voxel extent"),
            DEFAULT_VOXEL_EXTENT,
        )
        .with_min_value(MIN_VOXEL_EXTENT)
        .with_speed(0.01),
    )]
}

fn unary_child(
    id_map: &HashMap<NodeID, SDFNodeID>,
    children: &[Option<NodeID>],
) -> Option<SDFNodeID> {
    assert_eq!(children.len(), 1);
    children.first()?.map(|id| id_map[&id])
}

fn binary_children(
    id_map: &HashMap<NodeID, SDFNodeID>,
    children: &[Option<NodeID>],
) -> Option<(SDFNodeID, SDFNodeID)> {
    assert_eq!(children.len(), 2);
    let child_0 = children.first()?.map(|id| id_map[&id])?;
    let child_1 = children.get(1)?.map(|id| id_map[&id])?;
    Some((child_0, child_1))
}
