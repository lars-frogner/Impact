use super::{
    Node, NodeID,
    node_kind::{self},
};
use allocator_api2::{alloc::Allocator, vec::Vec as AVec};
use impact::impact_containers::HashMap;
use impact_voxel::{
    generation::{
        SDFVoxelGenerator,
        sdf::{SDFGenerator, SDFGeneratorBuilder, SDFNodeID},
        voxel_type::{SameVoxelTypeGenerator, VoxelTypeGenerator},
    },
    voxel_types::VoxelType,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
struct BuiltSDFGenerator {
    voxel_extent: f32,
    sdf_generator: SDFGenerator,
}

#[derive(Debug)]
enum SDFBuildOperation<'a> {
    VisitChildren((NodeID, &'a Node)),
    BuildNode((NodeID, &'a Node)),
}

pub fn build_sdf_voxel_generator<A>(
    arena: A,
    nodes: &BTreeMap<NodeID, Node>,
) -> Option<SDFVoxelGenerator>
where
    A: Allocator + Copy,
{
    let BuiltSDFGenerator {
        voxel_extent,
        sdf_generator,
    } = build_sdf_generator(arena, nodes)?;

    let voxel_type_generator =
        VoxelTypeGenerator::Same(SameVoxelTypeGenerator::new(VoxelType::from_idx(0)));

    Some(SDFVoxelGenerator::new(
        f64::from(voxel_extent),
        sdf_generator,
        voxel_type_generator,
    ))
}

pub fn default_sdf_voxel_generator() -> SDFVoxelGenerator {
    let voxel_extent = node_kind::DEFAULT_VOXEL_EXTENT;
    let sdf_generator = SDFGenerator::empty();

    let voxel_type_generator =
        VoxelTypeGenerator::Same(SameVoxelTypeGenerator::new(VoxelType::from_idx(0)));

    SDFVoxelGenerator::new(f64::from(voxel_extent), sdf_generator, voxel_type_generator)
}

fn build_sdf_generator<A>(arena: A, nodes: &BTreeMap<NodeID, Node>) -> Option<BuiltSDFGenerator>
where
    A: Allocator + Copy,
{
    let output_node = nodes.get(&0)?;

    let voxel_extent = node_kind::get_voxel_extent_from_output_node(&output_node.data.params);

    let root_node_id = output_node.children[0]?;
    let root_node = &nodes[&root_node_id];

    let mut builder = SDFGeneratorBuilder::with_capacity_in(nodes.len(), arena);

    let mut id_map = HashMap::<NodeID, SDFNodeID>::default();

    let mut operation_stack = AVec::new_in(arena);
    operation_stack.push(SDFBuildOperation::VisitChildren((root_node_id, root_node)));

    while let Some(operation) = operation_stack.pop() {
        match operation {
            SDFBuildOperation::VisitChildren((node_id, node)) => {
                if id_map.contains_key(&node_id) {
                    continue;
                }

                operation_stack.push(SDFBuildOperation::BuildNode((node_id, node)));

                for child_node_id in node.children.iter().rev() {
                    let child_node_id = (*child_node_id)?;
                    let child_node = &nodes[&child_node_id];
                    operation_stack.push(SDFBuildOperation::VisitChildren((
                        child_node_id,
                        child_node,
                    )));
                }
            }
            SDFBuildOperation::BuildNode((node_id, node)) => {
                let generator_node = node.data.kind.build_sdf_generator_node(
                    &id_map,
                    &node.children,
                    &node.data.params,
                )?;

                let sdf_node_id = builder.add_node(generator_node);
                id_map.insert(node_id, sdf_node_id);
            }
        }
    }

    Some(BuiltSDFGenerator {
        voxel_extent,
        sdf_generator: builder.build_with_arena(arena).unwrap(),
    })
}
