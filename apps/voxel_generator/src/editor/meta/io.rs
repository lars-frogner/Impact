use super::{
    MetaEnumParam, MetaFloatParam, MetaFloatRangeParam, MetaNode, MetaNodeChildLinks, MetaNodeData,
    MetaNodeID, MetaNodeParam, MetaNodeParentLinks, MetaUIntParam, MetaUIntRangeParam,
    node_kind::MetaNodeKind,
};
use anyhow::{Error, bail};
use impact::impact_containers::HashSet;
use serde::{Deserialize, Serialize};
use tinyvec::TinyVec;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IOMetaGraph {
    pub kind: IOMetaGraphKind,
    pub nodes: Vec<IOMetaNode>,
    pub collapsed_nodes: HashSet<MetaNodeID>,
}

#[derive(Clone, Debug, Serialize)]
pub struct IOMetaGraphRef<'a> {
    pub kind: IOMetaGraphKind,
    pub nodes: &'a [IOMetaNode],
    pub collapsed_nodes: &'a HashSet<MetaNodeID>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IOMetaGraphKind {
    Full { pan: [f32; 2], zoom: f32 },
    Subgraph { root_node_id: MetaNodeID },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IOMetaNode {
    pub id: MetaNodeID,
    pub position: (f32, f32),
    pub name: String,
    pub kind: MetaNodeKind,
    pub params: IOMetaNodeParams,
    pub links_to_parents: MetaNodeParentLinks,
    pub links_to_children: MetaNodeChildLinks,
}

type IOMetaNodeParams = TinyVec<[IOMetaNodeParam; 12]>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IOMetaNodeParam {
    Enum {
        variants: TinyVec<[String; 2]>,
        value: String,
    },
    UInt(u32),
    Float(f32),
    UIntRange {
        low: u32,
        high: u32,
    },
    FloatRange {
        low: f32,
        high: f32,
    },
}

impl IOMetaGraphKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Full { .. } => "full graph",
            Self::Subgraph { .. } => "subgraph",
        }
    }
}

impl IOMetaNode {
    pub fn offset_ids(&mut self, offset: MetaNodeID) {
        self.id += offset;

        self.links_to_parents
            .iter_mut()
            .chain(self.links_to_children.iter_mut())
            .flatten()
            .for_each(|link| {
                link.to_node += offset;
            });
    }
}

impl<'a> From<(&'a MetaNodeID, &'a MetaNode)> for IOMetaNode {
    fn from((id, node): (&'a MetaNodeID, &'a MetaNode)) -> Self {
        Self {
            id: *id,
            position: (node.position.x, node.position.y),
            name: node.data.name.clone(),
            kind: node.data.kind,
            params: node.data.params.iter().map(IOMetaNodeParam::from).collect(),
            links_to_parents: node.links_to_parents.clone(),
            links_to_children: node.links_to_children.clone(),
        }
    }
}

impl TryFrom<IOMetaNode> for MetaNode {
    type Error = Error;

    fn try_from(node: IOMetaNode) -> Result<Self, Self::Error> {
        let mut params = node.kind.params();
        if node.params.len() != params.len() {
            bail!("Invalid number of parameters");
        }
        for (param, io_param) in params.iter_mut().zip(node.params) {
            match (param, io_param) {
                (
                    MetaNodeParam::UInt(MetaUIntParam { value, .. }),
                    IOMetaNodeParam::UInt(io_value),
                ) => {
                    *value = io_value;
                }
                (
                    MetaNodeParam::Float(MetaFloatParam { value, .. }),
                    IOMetaNodeParam::Float(io_value),
                ) => {
                    *value = io_value;
                }
                (
                    MetaNodeParam::UIntRange(MetaUIntRangeParam {
                        low_value,
                        high_value,
                        ..
                    }),
                    IOMetaNodeParam::UIntRange { low, high },
                ) => {
                    *low_value = low;
                    *high_value = high;
                }
                (
                    MetaNodeParam::FloatRange(MetaFloatRangeParam {
                        low_value,
                        high_value,
                        ..
                    }),
                    IOMetaNodeParam::FloatRange { low, high },
                ) => {
                    *low_value = low;
                    *high_value = high;
                }
                _ => {
                    bail!("Inconsistent parameter types");
                }
            }
        }

        let data = MetaNodeData::new(node.name, node.kind, params);

        if node.links_to_parents.is_empty() && !node.kind.is_output() {
            bail!("Non-output nodes must have at least one parent link");
        }
        if node.links_to_children.len() != node.kind.n_child_slots() {
            bail!("Invalid number of links to children");
        }

        Ok(Self::new_with_links(
            node.position.into(),
            data,
            node.links_to_parents,
            node.links_to_children,
        ))
    }
}

impl<'a> From<&'a MetaNodeParam> for IOMetaNodeParam {
    fn from(param: &'a MetaNodeParam) -> Self {
        match param {
            MetaNodeParam::Enum(MetaEnumParam {
                variants, value, ..
            }) => Self::Enum {
                variants: variants.iter().map(|v| (*v).to_string()).collect(),
                value: (*value).to_string(),
            },
            MetaNodeParam::UInt(MetaUIntParam { value, .. }) => Self::UInt(*value),
            MetaNodeParam::Float(MetaFloatParam { value, .. }) => Self::Float(*value),
            MetaNodeParam::UIntRange(MetaUIntRangeParam {
                low_value,
                high_value,
                ..
            }) => Self::UIntRange {
                low: *low_value,
                high: *high_value,
            },
            MetaNodeParam::FloatRange(MetaFloatRangeParam {
                low_value,
                high_value,
                ..
            }) => Self::FloatRange {
                low: *low_value,
                high: *high_value,
            },
        }
    }
}

impl Default for IOMetaNodeParam {
    fn default() -> Self {
        Self::UInt(0)
    }
}
