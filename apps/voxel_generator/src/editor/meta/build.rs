use super::{
    MetaNode, MetaNodeID,
    node_kind::{self},
};
use allocator_api2::{alloc::Allocator, vec::Vec as AVec};
use impact::impact_containers::HashMap;
use impact_voxel::{
    generation::{
        SDFVoxelGenerator,
        sdf::{
            SDFGenerator, SDFGraph,
            meta::{MetaSDFGraph, MetaSDFNodeID},
        },
        voxel_type::{SameVoxelTypeGenerator, VoxelTypeGenerator},
    },
    voxel_types::VoxelType,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct SDFGraphBuildResult<A: Allocator> {
    pub voxel_extent: f32,
    pub graph: SDFGraph<A>,
}

#[derive(Clone, Debug)]
pub struct BuildScratch {
    id_map: HashMap<MetaNodeID, MetaSDFNodeID>,
}

#[derive(Debug)]
enum SDFBuildOperation<'a> {
    VisitChildren((MetaNodeID, &'a MetaNode)),
    BuildNode((MetaNodeID, &'a MetaNode)),
}

impl BuildScratch {
    pub fn new() -> Self {
        Self {
            id_map: HashMap::default(),
        }
    }
}

pub fn build_sdf_voxel_generator<A>(
    arena: A,
    compiled_graph: SDFGraphBuildResult<A>,
) -> SDFVoxelGenerator
where
    A: Allocator + Copy,
{
    let sdf_generator = compiled_graph.graph.build_with_arena(arena).unwrap();

    let voxel_type_generator =
        VoxelTypeGenerator::Same(SameVoxelTypeGenerator::new(VoxelType::from_idx(0)));

    SDFVoxelGenerator::new(
        f64::from(compiled_graph.voxel_extent),
        sdf_generator,
        voxel_type_generator,
    )
}

pub fn default_sdf_voxel_generator() -> SDFVoxelGenerator {
    let voxel_extent = node_kind::DEFAULT_VOXEL_EXTENT;
    let sdf_generator = SDFGenerator::empty();

    let voxel_type_generator =
        VoxelTypeGenerator::Same(SameVoxelTypeGenerator::new(VoxelType::from_idx(0)));

    SDFVoxelGenerator::new(f64::from(voxel_extent), sdf_generator, voxel_type_generator)
}

pub fn build_sdf_graph<A>(
    arena: A,
    scratch: &mut BuildScratch,
    nodes: &BTreeMap<MetaNodeID, MetaNode>,
) -> Option<SDFGraphBuildResult<A>>
where
    A: Allocator + Copy,
{
    let output_node = nodes.get(&0)?;

    let (voxel_extent, seed) =
        node_kind::get_voxel_extent_and_seed_from_output_node(&output_node.data.params);

    let root_node_id = output_node.links_to_children[0]?.to_node;
    let root_node = &nodes[&root_node_id];

    let mut meta_graph = MetaSDFGraph::with_capacity_in(nodes.len(), arena, seed);

    scratch.id_map.clear();

    let mut operation_stack = AVec::new_in(arena);
    operation_stack.push(SDFBuildOperation::VisitChildren((root_node_id, root_node)));

    while let Some(operation) = operation_stack.pop() {
        match operation {
            SDFBuildOperation::VisitChildren((node_id, node)) => {
                if scratch.id_map.contains_key(&node_id) {
                    continue;
                }

                operation_stack.push(SDFBuildOperation::BuildNode((node_id, node)));

                for link_to_child in node.links_to_children.iter().rev() {
                    let child_node_id = (*link_to_child)?.to_node;
                    let child_node = &nodes[&child_node_id];
                    operation_stack.push(SDFBuildOperation::VisitChildren((
                        child_node_id,
                        child_node,
                    )));
                }
            }
            SDFBuildOperation::BuildNode((node_id, node)) => {
                let generator_node = node.data.kind.build_sdf_generator_node(
                    &scratch.id_map,
                    &node.links_to_children,
                    &node.data.params,
                )?;

                let sdf_node_id = meta_graph.add_node(generator_node);
                scratch.id_map.insert(node_id, sdf_node_id);
            }
        }
    }

    let graph = meta_graph
        .build(arena)
        .inspect_err(|err| {
            impact_log::error!("Invalid meta graph: {err}");
        })
        .ok()?;

    Some(SDFGraphBuildResult {
        voxel_extent,
        graph,
    })
}
