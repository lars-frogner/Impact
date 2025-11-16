use super::AtomicNode;
use allocator_api2::alloc::Allocator;
use impact_voxel::generation::sdf::{
    MultifractalNoiseSDFModifier, MultiscaleSphereSDFModifier, SDFGraph, SDFIntersection, SDFNode,
    SDFNodeID, SDFRotation, SDFScaling, SDFSubtraction, SDFTranslation, SDFUnion,
};

pub fn update_viewer_nodes<A: Allocator>(graph: &SDFGraph<A>, viewer_nodes: &mut Vec<AtomicNode>) {
    viewer_nodes.clear();
    viewer_nodes.extend(graph.nodes().iter().map(|node| match node {
        SDFNode::Box(node) => AtomicNode::for_box(node),
        SDFNode::Sphere(node) => AtomicNode::for_sphere(node),
        SDFNode::Capsule(node) => AtomicNode::for_capsule(node),
        SDFNode::GradientNoise(node) => AtomicNode::for_gradient_noise(node),
        SDFNode::Translation(node) => AtomicNode::for_translation(node),
        SDFNode::Rotation(node) => AtomicNode::for_rotation(node),
        SDFNode::Scaling(node) => AtomicNode::for_scaling(node),
        SDFNode::MultifractalNoise(node) => AtomicNode::for_multifractal_noise(node),
        SDFNode::MultiscaleSphere(node) => AtomicNode::for_multiscale_sphere(node),
        SDFNode::Union(node) => AtomicNode::for_union(node),
        SDFNode::Subtraction(node) => AtomicNode::for_subtraction(node),
        SDFNode::Intersection(node) => AtomicNode::for_intersection(node),
    }));

    for (idx, node) in graph.nodes().iter().enumerate() {
        match node {
            SDFNode::Box(_)
            | SDFNode::Sphere(_)
            | SDFNode::Capsule(_)
            | SDFNode::GradientNoise(_) => {}
            SDFNode::Translation(SDFTranslation { child_id, .. })
            | SDFNode::Rotation(SDFRotation { child_id, .. })
            | SDFNode::Scaling(SDFScaling { child_id, .. })
            | SDFNode::MultifractalNoise(MultifractalNoiseSDFModifier { child_id, .. })
            | SDFNode::MultiscaleSphere(MultiscaleSphereSDFModifier { child_id, .. }) => {
                let child_idx = *child_id as usize;
                viewer_nodes[child_idx].parents.push(idx as SDFNodeID);
            }

            SDFNode::Union(SDFUnion {
                child_1_id,
                child_2_id,
                ..
            })
            | SDFNode::Subtraction(SDFSubtraction {
                child_1_id,
                child_2_id,
                ..
            })
            | SDFNode::Intersection(SDFIntersection {
                child_1_id,
                child_2_id,
                ..
            }) => {
                let child_1_idx = *child_1_id as usize;
                let child_2_idx = *child_2_id as usize;
                viewer_nodes[child_1_idx].parents.push(idx as SDFNodeID);
                viewer_nodes[child_2_idx].parents.push(idx as SDFNodeID);
            }
        }
    }

    let output_id = viewer_nodes.len() as SDFNodeID;
    viewer_nodes.push(AtomicNode::new_output(graph.root_node_id()));
    viewer_nodes[graph.root_node_id() as usize]
        .parents
        .push(output_id);
}
