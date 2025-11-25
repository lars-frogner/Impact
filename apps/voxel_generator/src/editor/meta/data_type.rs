use super::{
    MetaNode, MetaNodeID, MetaNodeInputDataTypes, MetaPaletteColor, MetaPortShape,
    node_kind::{MetaChildPortKind, MetaNodeKind, MetaParentPortKind},
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConcreteEdgeDataType {
    SingleSDF,
    SDFGroup,
    Instances,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EdgeDataType {
    Concrete(ConcreteEdgeDataType),
    #[default]
    Undefined,
}

#[derive(Clone, Debug)]
pub struct DataTypeScratch {
    stack: Vec<DataTypeResolveOperation>,
}

#[derive(Clone, Copy, Debug)]
enum DataTypeResolveOperation {
    VisitChildren(MetaNodeID),
    ResolveDataTypes(MetaNodeID),
}

impl EdgeDataType {
    pub const fn color(&self) -> MetaPaletteColor {
        match self {
            Self::Concrete(ConcreteEdgeDataType::SingleSDF | ConcreteEdgeDataType::SDFGroup) => {
                MetaPaletteColor::yellow()
            }
            Self::Concrete(ConcreteEdgeDataType::Instances) => MetaPaletteColor::blue(),
            Self::Undefined => MetaPaletteColor::green(),
        }
    }

    pub const fn port_shape(&self) -> MetaPortShape {
        match self {
            Self::Concrete(ConcreteEdgeDataType::SingleSDF) => MetaPortShape::Circle,
            Self::Concrete(ConcreteEdgeDataType::SDFGroup | ConcreteEdgeDataType::Instances)
            | Self::Undefined => MetaPortShape::Square,
        }
    }

    pub const fn port_label(&self) -> &str {
        match self {
            Self::Concrete(ConcreteEdgeDataType::SingleSDF) => "SDF",
            Self::Concrete(ConcreteEdgeDataType::SDFGroup) => "SDF group",
            Self::Concrete(ConcreteEdgeDataType::Instances) => "Instances",
            Self::Undefined => "Not determined",
        }
    }

    pub fn connection_allowed(input_port: Self, output_port: Self) -> bool {
        #[allow(clippy::enum_glob_use)]
        use ConcreteEdgeDataType::*;
        matches!(
            (input_port, output_port),
            (Self::Concrete(SingleSDF), Self::Concrete(SingleSDF))
                | (
                    Self::Concrete(SDFGroup),
                    Self::Concrete(SingleSDF | SDFGroup)
                )
                | (Self::Concrete(Instances), Self::Concrete(Instances))
                | (Self::Undefined, _)
                | (_, Self::Undefined)
        )
    }
}

impl DataTypeScratch {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }
}

pub fn input_and_output_types_for_new_node(
    kind: MetaNodeKind,
) -> (MetaNodeInputDataTypes, EdgeDataType) {
    let mut input_data_types = MetaNodeInputDataTypes::new();

    for port_kind in kind.child_port_kinds() {
        let data_type = match port_kind {
            MetaChildPortKind::SingleSDF => EdgeDataType::Concrete(ConcreteEdgeDataType::SingleSDF),
            MetaChildPortKind::SDFGroup => EdgeDataType::Concrete(ConcreteEdgeDataType::SDFGroup),
            MetaChildPortKind::Instances => EdgeDataType::Concrete(ConcreteEdgeDataType::Instances),
            MetaChildPortKind::Any => EdgeDataType::Undefined,
        };
        input_data_types.push(data_type);
    }

    let output_data_type = match kind.parent_port_kind() {
        MetaParentPortKind::SingleSDF => EdgeDataType::Concrete(ConcreteEdgeDataType::SingleSDF),
        MetaParentPortKind::SDFGroup => EdgeDataType::Concrete(ConcreteEdgeDataType::SDFGroup),
        MetaParentPortKind::Instances => EdgeDataType::Concrete(ConcreteEdgeDataType::Instances),
        MetaParentPortKind::SameAsInput { .. } => EdgeDataType::Undefined,
    };

    (input_data_types, output_data_type)
}

pub fn update_edge_data_types(
    scratch: &mut DataTypeScratch,
    nodes: &mut BTreeMap<MetaNodeID, MetaNode>,
) {
    scratch.stack.clear();

    for (&node_id, node) in nodes.iter() {
        if node.links_to_parents.iter().flatten().count() == 0 {
            scratch
                .stack
                .push(DataTypeResolveOperation::VisitChildren(node_id));
        }
    }

    while let Some(operation) = scratch.stack.pop() {
        match operation {
            DataTypeResolveOperation::VisitChildren(node_id) => {
                let Some(node) = nodes.get(&node_id) else {
                    continue;
                };

                scratch
                    .stack
                    .push(DataTypeResolveOperation::ResolveDataTypes(node_id));

                for link in node.links_to_children.iter().flatten() {
                    scratch
                        .stack
                        .push(DataTypeResolveOperation::VisitChildren(link.to_node));
                }
            }
            DataTypeResolveOperation::ResolveDataTypes(node_id) => {
                let node = &nodes[&node_id];
                let node_kind = node.data.kind;

                let mut input_data_types = MetaNodeInputDataTypes::new();

                for (slot, port_kind) in node_kind.child_port_kinds().enumerate() {
                    let data_type = match port_kind {
                        MetaChildPortKind::SingleSDF => {
                            EdgeDataType::Concrete(ConcreteEdgeDataType::SingleSDF)
                        }
                        MetaChildPortKind::SDFGroup => {
                            EdgeDataType::Concrete(ConcreteEdgeDataType::SDFGroup)
                        }
                        MetaChildPortKind::Instances => {
                            EdgeDataType::Concrete(ConcreteEdgeDataType::Instances)
                        }
                        MetaChildPortKind::Any => {
                            if let Some(link) = node.links_to_children[slot]
                                && let Some(child_node) = nodes.get(&link.to_node)
                            {
                                child_node.output_data_type
                            } else {
                                EdgeDataType::Undefined
                            }
                        }
                    };
                    input_data_types.push(data_type);
                }

                let output_data_type = match node_kind.parent_port_kind() {
                    MetaParentPortKind::SingleSDF => {
                        EdgeDataType::Concrete(ConcreteEdgeDataType::SingleSDF)
                    }
                    MetaParentPortKind::SDFGroup => {
                        EdgeDataType::Concrete(ConcreteEdgeDataType::SDFGroup)
                    }
                    MetaParentPortKind::Instances => {
                        EdgeDataType::Concrete(ConcreteEdgeDataType::Instances)
                    }
                    MetaParentPortKind::SameAsInput { slot } => {
                        if let Some(link) = node.links_to_children[slot]
                            && let Some(child_node) = nodes.get(&link.to_node)
                            && EdgeDataType::connection_allowed(
                                input_data_types[slot],
                                child_node.output_data_type,
                            )
                        {
                            child_node.output_data_type
                        } else {
                            EdgeDataType::Undefined
                        }
                    }
                };

                let node = nodes.get_mut(&node_id).unwrap();
                node.output_data_type = output_data_type;
                node.input_data_types = input_data_types;
            }
        }
    }
}
