use crate::editor::meta::MetaNodeID;

use super::{
    MetaFloatParam, MetaFloatRangeParam, MetaNode, MetaNodeChildLinks, MetaNodeData, MetaNodeParam,
    MetaNodeParentLinks, MetaUIntParam, MetaUIntRangeParam, node_kind::MetaNodeKind,
};
use anyhow::{Error, bail};
use serde::{Deserialize, Serialize};
use tinyvec::TinyVec;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IOMetaNodeGraph {
    pub nodes: Vec<IOMetaNode>,
}

#[derive(Clone, Debug, Serialize)]
pub struct IOMetaNodeGraphRef<'a> {
    pub nodes: &'a [IOMetaNode],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IOMetaNode {
    pub id: MetaNodeID,
    pub position: (f32, f32),
    pub kind: MetaNodeKind,
    pub params: IOMetaNodeParams,
    pub links_to_parents: MetaNodeParentLinks,
    pub links_to_children: MetaNodeChildLinks,
}

type IOMetaNodeParams = TinyVec<[IOMetaNodeParam; 12]>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IOMetaNodeParam {
    UInt(u32),
    Float(f32),
    UIntRange { low: u32, high: u32 },
    FloatRange { low: f32, high: f32 },
}

impl<'a> From<(&'a MetaNodeID, &'a MetaNode)> for IOMetaNode {
    fn from((id, node): (&'a MetaNodeID, &'a MetaNode)) -> Self {
        Self {
            id: *id,
            position: (node.position.x, node.position.y),
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

        let data = MetaNodeData::new_with_params(node.kind, params);

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
