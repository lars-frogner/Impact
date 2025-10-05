//! Generation of spatial voxel distributions.

use crate::{Voxel, VoxelSignedDistance, voxel_types::VoxelType};
use allocator_api2::{
    alloc::{Allocator, Global},
    vec::Vec as AVec,
};
use anyhow::{Result, anyhow, bail};
use impact_geometry::{AxisAlignedBox, OrientedBox};
use nalgebra::{Point3, Quaternion, UnitQuaternion, UnitVector3, Vector3, point, vector};
use noise::{HybridMulti, MultiFractal, NoiseFn, Simplex};
use ordered_float::OrderedFloat;
use twox_hash::XxHash32;

/// Represents a voxel generator that provides a voxel type given the voxel
/// indices.
pub trait VoxelGenerator {
    /// Returns the extent of single voxel.
    fn voxel_extent(&self) -> f64;

    /// Returns the number of voxels along the x-, y- and z-axis of the grid,
    /// respectively.
    fn grid_shape(&self) -> [usize; 3];

    /// Returns the voxel at the given indices in a voxel grid. If the indices
    /// are outside the bounds of the grid, this should return
    /// [`Voxel::maximally_outside`].
    fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Voxel;
}

/// Generator for a voxel object from a signed distance field.
#[derive(Clone, Debug)]
pub struct SDFVoxelGenerator {
    voxel_extent: f64,
    grid_shape: [usize; 3],
    shifted_grid_center: Point3<f32>,
    sdf_generator: SDFGenerator,
    voxel_type_generator: VoxelTypeGenerator,
}

/// A signed distance field generator.
///
/// # Note
/// We might not actually want a real signed distance field, because it is hard
/// to modify it efficiently without invalidating distances away from the
/// surface. Instead, it might be better to embrace it as a signed field that
/// has correct distances only close to the surface, as this is what we
/// typically care about.
#[derive(Clone, Debug)]
pub struct SDFGenerator {
    /// Nodes in reverse depth-first order. The last node is the root.
    nodes: Vec<SDFGeneratorNode>,
    domain: AxisAlignedBox<f32>,
}

#[derive(Clone, Debug)]
pub struct SDFGeneratorBuilder<A: Allocator = Global> {
    nodes: AVec<SDFGeneratorNode, A>,
    root_node_id: SDFNodeID,
}

pub type SDFNodeID = u32;

#[derive(Clone, Debug)]
pub enum SDFGeneratorNode {
    // Primitives
    Box(BoxSDFGenerator),
    Sphere(SphereSDFGenerator),
    GradientNoise(GradientNoiseSDFGenerator),

    // Transforms
    Translation(SDFTranslation),
    Rotation(SDFRotation),
    Scaling(SDFScaling),

    // Modifiers
    MultifractalNoise(MultifractalNoiseSDFModifier),
    MultiscaleSphere(MultiscaleSphereSDFModifier),

    // Binary operations
    Union(SDFUnion),
    Subtraction(SDFSubtraction),
    Intersection(SDFIntersection),
}

/// Generator for a signed distance field representing a box.
#[derive(Clone, Debug)]
pub struct BoxSDFGenerator {
    half_extents: Vector3<f32>,
}

/// Generator for a signed distance field representing a sphere.
#[derive(Clone, Debug)]
pub struct SphereSDFGenerator {
    radius: f32,
}

/// Generator for a signed "distance" field obtained by thresholding a gradient
/// noise pattern.
#[derive(Clone, Debug)]
pub struct GradientNoiseSDFGenerator {
    half_extents: Vector3<f32>,
    noise_frequency: f32,
    noise_threshold: f32,
    noise: Simplex,
}

#[derive(Clone, Debug)]
pub struct SDFTranslation {
    pub child_id: SDFNodeID,
    pub translation: Vector3<f32>,
}

#[derive(Clone, Debug)]
pub struct SDFRotation {
    pub child_id: SDFNodeID,
    pub rotation: UnitQuaternion<f32>,
}

#[derive(Clone, Debug)]
pub struct SDFScaling {
    pub child_id: SDFNodeID,
    pub scaling: f32,
}

#[derive(Clone, Debug)]
pub struct SDFUnion {
    pub child_1_id: SDFNodeID,
    pub child_2_id: SDFNodeID,
    pub smoothness: f32,
}

#[derive(Clone, Debug)]
pub struct SDFSubtraction {
    pub child_1_id: SDFNodeID,
    pub child_2_id: SDFNodeID,
    pub smoothness: f32,
}

#[derive(Clone, Debug)]
pub struct SDFIntersection {
    pub child_1_id: SDFNodeID,
    pub child_2_id: SDFNodeID,
    pub smoothness: f32,
}

/// Modifier for a signed distance field that adds a multifractal noise term to
/// the signed distance.
///
/// Note that the resulting field will in general not contain correct distances,
/// so this is best used only for minor perturbations.
#[derive(Clone, Debug)]
pub struct MultifractalNoiseSDFModifier {
    child_id: SDFNodeID,
    noise: HybridMulti<Simplex>,
    amplitude: f32,
}

/// Modifier for a signed distance field that performs a stochastic multiscale
/// modification of the signed distance around the surface. This is done by
/// superimposing a field representing a grid of spheres with randomized radii,
/// which is unioned with the original field aroud the surface. This is repeated
/// for each octave with successively smaller and more numerous spheres.
///
/// See <https://iquilezles.org/articles/fbmsdf/> for more information.
///
/// The output will be a valid signed distance field.
#[derive(Clone, Debug)]
pub struct MultiscaleSphereSDFModifier {
    child_id: SDFNodeID,
    octaves: u32,
    frequency: f32,
    persistence: f32,
    inflation: f32,
    smoothness: f32,
    seed: u32,
}

#[derive(Clone, Debug)]
enum BuildOperation {
    VisitChildren(SDFNodeID),
    Process(SDFNodeID),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NodeBuildState {
    Unvisited,
    ChildrenBeingVisited,
    DomainDetermined,
}

#[allow(clippy::large_enum_variant)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug)]
pub enum VoxelTypeGenerator {
    Same(SameVoxelTypeGenerator),
    GradientNoise(GradientNoiseVoxelTypeGenerator),
}

/// Voxel type generator that always returns the same voxel type.
#[derive(Clone, Debug)]
pub struct SameVoxelTypeGenerator {
    voxel_type: VoxelType,
}

/// Voxel type generator that determines voxel types by generating a 4D
/// gradient noise pattern and selecting the voxel type for which the fourth
/// component of the noise is strongest at each location.
#[derive(Clone, Debug)]
pub struct GradientNoiseVoxelTypeGenerator {
    voxel_types: Vec<VoxelType>,
    noise_frequency: f64,
    noise_scale_for_voxel_type_dim: f64,
    noise: Simplex,
}

impl SDFVoxelGenerator {
    /// Creates a new voxel generator using the given signed distance field
    /// and voxel type generators.
    pub fn new(
        voxel_extent: f64,
        sdf_generator: SDFGenerator,
        voxel_type_generator: VoxelTypeGenerator,
    ) -> Self {
        assert!(voxel_extent > 0.0);

        let sdf_domain = sdf_generator.domain();
        let sdf_domain_extents: [_; 3] = sdf_domain.extents().into();

        if sdf_domain_extents.contains(&0.0) {
            return Self {
                voxel_extent,
                grid_shape: [0; 3],
                shifted_grid_center: [-0.5; 3].into(),
                sdf_generator,
                voxel_type_generator,
            };
        }

        // Make room for a border of empty voxels around the object to so that
        // the surface nets meshing algorithm can correctly interpolate
        // distances at the boundaries
        let grid_shape = sdf_domain_extents.map(|extent| {
            let extent = extent.ceil() as usize;
            // Add a one-voxel border on each side
            extent + 2
        });

        let grid_center_relative_to_domain_lower_corner =
            Point3::from(grid_shape.map(|n| 0.5 * n as f32));

        // Since the domain can be translated relative to the origin of the SDF
        // reference frame, we subtract the domain center to get the grid center
        // relative to the origin
        let grid_center_relative_to_sdf_origin =
            grid_center_relative_to_domain_lower_corner - sdf_domain.center().coords;

        // The center here is offset by half a grid cell relative to the coordinates
        // in the voxel object to account for the fact that we want to evaluate the
        // SDF at the center of each voxel
        let shifted_grid_center_relative_to_sdf_origin =
            grid_center_relative_to_sdf_origin.map(|coord| coord - 0.5);

        Self {
            voxel_extent,
            grid_shape,
            shifted_grid_center: shifted_grid_center_relative_to_sdf_origin,
            sdf_generator,
            voxel_type_generator,
        }
    }

    /// Returns the center of the voxel grid in the SDF reference frame. The
    /// coordinates are in whole voxels.
    pub fn grid_center(&self) -> Point3<f32> {
        self.shifted_grid_center.map(|coord| coord + 0.5) // Unshift
    }
}

impl VoxelGenerator for SDFVoxelGenerator {
    fn voxel_extent(&self) -> f64 {
        self.voxel_extent
    }

    fn grid_shape(&self) -> [usize; 3] {
        self.grid_shape
    }

    fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Voxel {
        let displacement_from_center =
            point![i as f32, j as f32, k as f32] - self.shifted_grid_center;

        let signed_distance = VoxelSignedDistance::from_f32(
            self.sdf_generator
                .compute_signed_distance(&displacement_from_center),
        );

        if signed_distance.is_negative() {
            let voxel_type = self.voxel_type_generator.voxel_type_at_indices(i, j, k);
            Voxel::non_empty(voxel_type, signed_distance)
        } else {
            Voxel::empty(signed_distance)
        }
    }
}

impl SDFGenerator {
    const MAX_PRIMITIVES: usize = 16;

    pub fn empty() -> Self {
        Self {
            nodes: Vec::new(),
            domain: AxisAlignedBox::new(Point3::origin(), Point3::origin()),
        }
    }

    pub fn new<A, AN>(
        arena: A,
        nodes: AVec<SDFGeneratorNode, AN>,
        root_node_id: SDFNodeID,
    ) -> Result<Self>
    where
        A: Allocator + Copy,
        AN: Allocator,
    {
        let mut ordered_nodes = Vec::with_capacity(nodes.len());

        let mut domains = AVec::new_in(arena);
        domains.resize(nodes.len(), zero_domain());

        let mut states = AVec::new_in(arena);
        states.resize(nodes.len(), NodeBuildState::Unvisited);

        let mut operation_stack = AVec::with_capacity_in(3 * nodes.len(), arena);

        operation_stack.push(BuildOperation::VisitChildren(root_node_id));

        while let Some(operation) = operation_stack.pop() {
            match operation {
                BuildOperation::VisitChildren(node_id) => {
                    let node_idx = node_id as usize;

                    let state = states
                        .get_mut(node_idx)
                        .ok_or_else(|| anyhow!("Missing SDF node {node_id}"))?;

                    operation_stack.push(BuildOperation::Process(node_id));

                    match *state {
                        NodeBuildState::DomainDetermined => {
                            // Domain already determined via a different parent
                        }
                        NodeBuildState::ChildrenBeingVisited => {
                            // We got back to the same node while visiting its children
                            bail!("Detected cycle in SDF generator node graph")
                        }
                        NodeBuildState::Unvisited => {
                            *state = NodeBuildState::ChildrenBeingVisited;

                            match &nodes[node_idx] {
                                SDFGeneratorNode::Box(_)
                                | SDFGeneratorNode::Sphere(_)
                                | SDFGeneratorNode::GradientNoise(_) => {}
                                SDFGeneratorNode::Translation(SDFTranslation {
                                    child_id, ..
                                })
                                | SDFGeneratorNode::Rotation(SDFRotation { child_id, .. })
                                | SDFGeneratorNode::Scaling(SDFScaling { child_id, .. })
                                | SDFGeneratorNode::MultifractalNoise(
                                    MultifractalNoiseSDFModifier { child_id, .. },
                                )
                                | SDFGeneratorNode::MultiscaleSphere(
                                    MultiscaleSphereSDFModifier { child_id, .. },
                                ) => {
                                    operation_stack.push(BuildOperation::VisitChildren(*child_id));
                                }
                                SDFGeneratorNode::Union(SDFUnion {
                                    child_1_id,
                                    child_2_id,
                                    ..
                                })
                                | SDFGeneratorNode::Subtraction(SDFSubtraction {
                                    child_1_id,
                                    child_2_id,
                                    ..
                                })
                                | SDFGeneratorNode::Intersection(SDFIntersection {
                                    child_1_id,
                                    child_2_id,
                                    ..
                                }) => {
                                    operation_stack
                                        .push(BuildOperation::VisitChildren(*child_1_id));
                                    operation_stack
                                        .push(BuildOperation::VisitChildren(*child_2_id));
                                }
                            }
                        }
                    }
                }
                BuildOperation::Process(node_id) => {
                    let node_idx = node_id as usize;
                    let node = &nodes[node_idx];
                    let state = &mut states[node_idx];

                    ordered_nodes.push(node.clone());

                    if *state != NodeBuildState::DomainDetermined {
                        *state = NodeBuildState::DomainDetermined;

                        match node {
                            SDFGeneratorNode::Box(box_generator) => {
                                domains[node_idx] = box_generator.domain_bounds();
                            }
                            SDFGeneratorNode::Sphere(sphere_generator) => {
                                domains[node_idx] = sphere_generator.domain_bounds();
                            }
                            SDFGeneratorNode::GradientNoise(gradient_noise_generator) => {
                                domains[node_idx] = gradient_noise_generator.domain_bounds();
                            }
                            &SDFGeneratorNode::Translation(SDFTranslation {
                                child_id,
                                translation,
                            }) => {
                                let child_domain = &domains[child_id as usize];
                                domains[node_idx] = child_domain.translated(&translation);
                            }
                            &SDFGeneratorNode::Rotation(SDFRotation { child_id, rotation }) => {
                                let child_domain = &domains[child_id as usize];
                                let domain_ob = OrientedBox::from_axis_aligned_box(child_domain)
                                    .rotated(&rotation);
                                domains[node_idx] = AxisAlignedBox::aabb_for_point_array(
                                    &domain_ob.compute_corners(),
                                );
                            }
                            &SDFGeneratorNode::Scaling(SDFScaling { child_id, scaling }) => {
                                let child_domain = &domains[child_id as usize];
                                domains[node_idx] = child_domain.scaled_about_center(scaling);
                            }
                            SDFGeneratorNode::MultifractalNoise(MultifractalNoiseSDFModifier {
                                child_id,
                                amplitude,
                                ..
                            }) => {
                                let child_domain = &domains[*child_id as usize];
                                domains[node_idx] = pad_domain(child_domain.clone(), *amplitude);
                            }
                            SDFGeneratorNode::MultiscaleSphere(
                                modifier @ MultiscaleSphereSDFModifier { child_id, .. },
                            ) => {
                                let child_domain = &domains[*child_id as usize];
                                domains[node_idx] =
                                    pad_domain(child_domain.clone(), modifier.max_scale());
                            }
                            &SDFGeneratorNode::Union(SDFUnion {
                                child_1_id,
                                child_2_id,
                                smoothness,
                            }) => {
                                let child_1_domain = &domains[child_1_id as usize];
                                let child_2_domain = &domains[child_2_id as usize];
                                let domain =
                                    AxisAlignedBox::aabb_from_pair(child_1_domain, child_2_domain);
                                domains[node_idx] =
                                    pad_domain(domain, domain_padding_for_smoothness(smoothness));
                            }
                            &SDFGeneratorNode::Subtraction(SDFSubtraction {
                                child_1_id,
                                child_2_id: _,
                                smoothness,
                            }) => {
                                let selected_child_domain = &domains[child_1_id as usize];
                                domains[node_idx] = pad_domain(
                                    selected_child_domain.clone(),
                                    domain_padding_for_smoothness(smoothness),
                                );
                            }
                            &SDFGeneratorNode::Intersection(SDFIntersection {
                                child_1_id,
                                child_2_id,
                                smoothness,
                            }) => {
                                let child_1_domain = &domains[child_1_id as usize];
                                let child_2_domain = &domains[child_2_id as usize];
                                domains[node_idx] = if let Some(domain) =
                                    child_1_domain.compute_overlap_with(child_2_domain)
                                {
                                    pad_domain(domain, domain_padding_for_smoothness(smoothness))
                                } else {
                                    zero_domain()
                                };
                            }
                        }
                    }
                }
            }
        }

        let domain = domains[root_node_id as usize].clone();

        Ok(Self {
            nodes: ordered_nodes,
            domain,
        })
    }

    /// Returns the domain where the signed distance field can be negative, in
    /// voxel grid coordinates relative to the origin of the SDF reference
    /// frame. If the domain is not translated, the origin coincides with the
    /// center of the domain.
    pub fn domain(&self) -> &AxisAlignedBox<f32> {
        &self.domain
    }

    // Computes the signed distance at the given displacement in voxel grid
    // coordinates from the center of the field.
    #[inline]
    pub fn compute_signed_distance(&self, displacement_from_center: &Vector3<f32>) -> f32 {
        if self.nodes.is_empty() {
            return VoxelSignedDistance::MAX_F32;
        }

        let mut displacements_for_primitives = [Vector3::zeros(); Self::MAX_PRIMITIVES];
        let mut branch_idx: usize = 0;

        displacements_for_primitives[branch_idx] = *displacement_from_center;

        for node in self.nodes.iter().rev() {
            match node {
                SDFGeneratorNode::Union(_)
                | SDFGeneratorNode::Subtraction(_)
                | SDFGeneratorNode::Intersection(_) => {
                    // Duplicate current displacement for the second child branch
                    displacements_for_primitives[branch_idx + 1] =
                        displacements_for_primitives[branch_idx];
                }
                SDFGeneratorNode::Translation(SDFTranslation { translation, .. }) => {
                    displacements_for_primitives[branch_idx] -= translation;
                }
                SDFGeneratorNode::Rotation(SDFRotation { rotation, .. }) => {
                    let displacement = &mut displacements_for_primitives[branch_idx];
                    *displacement = rotation.inverse_transform_vector(displacement);
                }
                SDFGeneratorNode::Scaling(SDFScaling { scaling, .. }) => {
                    displacements_for_primitives[branch_idx].unscale_mut(*scaling);
                }
                SDFGeneratorNode::Box(_)
                | SDFGeneratorNode::Sphere(_)
                | SDFGeneratorNode::GradientNoise(_) => {
                    branch_idx += 1;
                }
                SDFGeneratorNode::MultifractalNoise(_) | SDFGeneratorNode::MultiscaleSphere(_) => {}
            }
        }

        let mut signed_distance_stack = [0.0_f32; Self::MAX_PRIMITIVES];
        let mut primitive_index_stack = [0_usize; Self::MAX_PRIMITIVES];
        let mut stack_top: usize = 0;

        for node in &self.nodes {
            match node {
                SDFGeneratorNode::Box(box_generator) => {
                    debug_assert!(branch_idx > 0);
                    branch_idx -= 1;

                    let displacement = &displacements_for_primitives[branch_idx];

                    signed_distance_stack[stack_top] =
                        box_generator.compute_signed_distance(displacement);
                    primitive_index_stack[stack_top] = branch_idx;

                    stack_top += 1;
                }
                SDFGeneratorNode::Sphere(sphere_generator) => {
                    debug_assert!(branch_idx > 0);
                    branch_idx -= 1;

                    let displacement = &displacements_for_primitives[branch_idx];

                    signed_distance_stack[stack_top] =
                        sphere_generator.compute_signed_distance(displacement);
                    primitive_index_stack[stack_top] = branch_idx;

                    stack_top += 1;
                }
                SDFGeneratorNode::GradientNoise(gradient_noise_generator) => {
                    debug_assert!(branch_idx > 0);
                    branch_idx -= 1;

                    let displacement = &displacements_for_primitives[branch_idx];

                    signed_distance_stack[stack_top] =
                        gradient_noise_generator.compute_signed_distance(displacement);
                    primitive_index_stack[stack_top] = branch_idx;

                    stack_top += 1;
                }
                SDFGeneratorNode::Translation(_) | SDFGeneratorNode::Rotation(_) => {}
                SDFGeneratorNode::Scaling(SDFScaling { scaling, .. }) => {
                    debug_assert!(stack_top >= 1);
                    signed_distance_stack[stack_top - 1] *= scaling;
                }
                SDFGeneratorNode::MultifractalNoise(modifier) => {
                    let primitive_index = primitive_index_stack[stack_top - 1];
                    let displacement = &displacements_for_primitives[primitive_index];
                    let perturbation = modifier.compute_signed_distance_perturbation(displacement);
                    signed_distance_stack[stack_top - 1] += perturbation;
                }
                SDFGeneratorNode::MultiscaleSphere(modifier) => {
                    let primitive_index = primitive_index_stack[stack_top - 1];
                    let displacement = &displacements_for_primitives[primitive_index];
                    let signed_distance = &mut signed_distance_stack[stack_top - 1];
                    *signed_distance =
                        modifier.modify_signed_distance(displacement, *signed_distance);
                }
                &SDFGeneratorNode::Union(SDFUnion { smoothness, .. }) => {
                    debug_assert!(stack_top >= 2);
                    let distance_1 = signed_distance_stack[stack_top - 2];
                    let distance_2 = signed_distance_stack[stack_top - 1];
                    stack_top -= 1;
                    signed_distance_stack[stack_top - 1] =
                        sdf_union(distance_1, distance_2, smoothness);
                    // primitive_index_stack[stack_top - 1] already holds a valid displacement index
                }
                &SDFGeneratorNode::Subtraction(SDFSubtraction { smoothness, .. }) => {
                    debug_assert!(stack_top >= 2);
                    let distance_1 = signed_distance_stack[stack_top - 2];
                    let distance_2 = signed_distance_stack[stack_top - 1];
                    stack_top -= 1;
                    signed_distance_stack[stack_top - 1] =
                        sdf_subtraction(distance_1, distance_2, smoothness);
                }
                &SDFGeneratorNode::Intersection(SDFIntersection { smoothness, .. }) => {
                    debug_assert!(stack_top >= 2);
                    let distance_1 = signed_distance_stack[stack_top - 2];
                    let distance_2 = signed_distance_stack[stack_top - 1];
                    stack_top -= 1;
                    signed_distance_stack[stack_top - 1] =
                        sdf_intersection(distance_1, distance_2, smoothness);
                }
            }
        }

        assert_eq!(stack_top, 1);

        signed_distance_stack[0]
    }
}

impl Default for SDFGenerator {
    fn default() -> Self {
        Self::empty()
    }
}

impl From<BoxSDFGenerator> for SDFGenerator {
    fn from(generator: BoxSDFGenerator) -> Self {
        let mut nodes = AVec::new();
        nodes.push(SDFGeneratorNode::Box(generator));
        Self::new(Global, nodes, 0).unwrap()
    }
}

impl From<SphereSDFGenerator> for SDFGenerator {
    fn from(generator: SphereSDFGenerator) -> Self {
        let mut nodes = AVec::new();
        nodes.push(SDFGeneratorNode::Sphere(generator));
        Self::new(Global, nodes, 0).unwrap()
    }
}

impl From<GradientNoiseSDFGenerator> for SDFGenerator {
    fn from(generator: GradientNoiseSDFGenerator) -> Self {
        let mut nodes = AVec::new();
        nodes.push(SDFGeneratorNode::GradientNoise(generator));
        Self::new(Global, nodes, 0).unwrap()
    }
}

impl<A: Allocator> SDFGeneratorBuilder<A> {
    pub fn new_in(alloc: A) -> Self {
        Self::with_capacity_in(0, alloc)
    }

    pub fn with_capacity_in(capacity: usize, alloc: A) -> Self {
        Self {
            nodes: AVec::<_, A>::with_capacity_in(capacity, alloc),
            root_node_id: 0,
        }
    }

    pub fn build(self) -> Result<SDFGenerator> {
        self.build_with_arena(Global)
    }

    pub fn build_with_arena<AR>(self, arena: AR) -> Result<SDFGenerator>
    where
        AR: Allocator + Copy,
    {
        if self.nodes.is_empty() {
            Ok(SDFGenerator::empty())
        } else {
            SDFGenerator::new(arena, self.nodes, self.root_node_id)
        }
    }

    pub fn add_box(&mut self, extents: [f32; 3]) -> SDFNodeID {
        self.add_node_and_set_to_root(SDFGeneratorNode::Box(BoxSDFGenerator::new(extents)))
    }

    pub fn add_sphere(&mut self, radius: f32) -> SDFNodeID {
        self.add_node_and_set_to_root(SDFGeneratorNode::Sphere(SphereSDFGenerator::new(radius)))
    }

    pub fn add_gradient_noise(
        &mut self,
        extents: [f32; 3],
        noise_frequency: f32,
        noise_threshold: f32,
        seed: u32,
    ) -> SDFNodeID {
        self.add_node_and_set_to_root(SDFGeneratorNode::GradientNoise(
            GradientNoiseSDFGenerator::new(extents, noise_frequency, noise_threshold, seed),
        ))
    }

    pub fn add_translation(&mut self, child_id: SDFNodeID, translation: Vector3<f32>) -> SDFNodeID {
        self.add_node_and_set_to_root(SDFGeneratorNode::Translation(SDFTranslation {
            child_id,
            translation,
        }))
    }

    pub fn add_rotation(
        &mut self,
        child_id: SDFNodeID,
        rotation: UnitQuaternion<f32>,
    ) -> SDFNodeID {
        self.add_node_and_set_to_root(SDFGeneratorNode::Rotation(SDFRotation {
            child_id,
            rotation,
        }))
    }

    pub fn add_scaling(&mut self, child_id: SDFNodeID, scaling: f32) -> SDFNodeID {
        self.add_node_and_set_to_root(SDFGeneratorNode::Scaling(SDFScaling::new(
            child_id, scaling,
        )))
    }

    pub fn add_multifractal_noise(
        &mut self,
        child_id: SDFNodeID,
        octaves: u32,
        frequency: f32,
        lacunarity: f32,
        persistence: f32,
        amplitude: f32,
        seed: u32,
    ) -> SDFNodeID {
        self.add_node_and_set_to_root(SDFGeneratorNode::MultifractalNoise(
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

    pub fn add_multiscale_sphere(
        &mut self,
        child_id: SDFNodeID,
        octaves: u32,
        max_scale: f32,
        persistence: f32,
        inflation: f32,
        smoothness: f32,
        seed: u32,
    ) -> SDFNodeID {
        self.add_node_and_set_to_root(SDFGeneratorNode::MultiscaleSphere(
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

    pub fn add_union(
        &mut self,
        child_1_id: SDFNodeID,
        child_2_id: SDFNodeID,
        smoothness: f32,
    ) -> SDFNodeID {
        self.add_node_and_set_to_root(SDFGeneratorNode::Union(SDFUnion::new(
            child_1_id, child_2_id, smoothness,
        )))
    }

    pub fn add_subtraction(
        &mut self,
        child_1_id: SDFNodeID,
        child_2_id: SDFNodeID,
        smoothness: f32,
    ) -> SDFNodeID {
        self.add_node_and_set_to_root(SDFGeneratorNode::Subtraction(SDFSubtraction::new(
            child_1_id, child_2_id, smoothness,
        )))
    }

    pub fn add_intersection(
        &mut self,
        child_1_id: SDFNodeID,
        child_2_id: SDFNodeID,
        smoothness: f32,
    ) -> SDFNodeID {
        self.add_node_and_set_to_root(SDFGeneratorNode::Intersection(SDFIntersection::new(
            child_1_id, child_2_id, smoothness,
        )))
    }

    pub fn add_node_and_set_to_root(&mut self, node: SDFGeneratorNode) -> SDFNodeID {
        let node_id = self.nodes.len().try_into().unwrap();
        self.nodes.push(node);
        self.root_node_id = node_id;
        node_id
    }
}

impl SDFGeneratorBuilder<Global> {
    pub fn new() -> Self {
        Self::new_in(Global)
    }
}

impl Default for SDFGeneratorBuilder<Global> {
    fn default() -> Self {
        Self::new()
    }
}

impl BoxSDFGenerator {
    /// Creates a new generator for a box with the given extents (in voxels).
    pub fn new(extents: [f32; 3]) -> Self {
        assert!(extents.iter().copied().all(f32::is_sign_positive));
        let half_extents = 0.5 * Vector3::from(extents);
        Self { half_extents }
    }

    fn domain_bounds(&self) -> AxisAlignedBox<f32> {
        AxisAlignedBox::new((-self.half_extents).into(), self.half_extents.into())
    }

    fn compute_signed_distance(&self, displacement_from_center: &Vector3<f32>) -> f32 {
        let q = displacement_from_center.abs() - self.half_extents;
        q.sup(&Vector3::zeros()).magnitude() + f32::min(q.max(), 0.0)
    }
}

impl SphereSDFGenerator {
    /// Creates a new generator for a sphere with the given radius (in voxels).
    pub fn new(radius: f32) -> Self {
        assert!(radius >= 0.0);
        Self { radius }
    }

    fn domain_bounds(&self) -> AxisAlignedBox<f32> {
        AxisAlignedBox::new([-self.radius; 3].into(), [self.radius; 3].into())
    }

    fn compute_signed_distance(&self, displacement_from_center: &Vector3<f32>) -> f32 {
        displacement_from_center.magnitude() - self.radius
    }
}

impl GradientNoiseSDFGenerator {
    /// Creates a new generator for a gradient noise voxel pattern with the
    /// given extents (in voxels), noise frequency, noise threshold and seed.
    pub fn new(extents: [f32; 3], noise_frequency: f32, noise_threshold: f32, seed: u32) -> Self {
        assert!(extents.iter().copied().all(f32::is_sign_positive));
        let half_extents = 0.5 * Vector3::from(extents);
        let noise = Simplex::new(seed);
        Self {
            half_extents,
            noise_frequency,
            noise_threshold,
            noise,
        }
    }

    fn domain_bounds(&self) -> AxisAlignedBox<f32> {
        AxisAlignedBox::new((-self.half_extents).into(), self.half_extents.into())
    }

    fn compute_signed_distance(&self, displacement_from_center: &Vector3<f32>) -> f32 {
        let noise_point: [f64; 3] = (self.noise_frequency * displacement_from_center)
            .cast()
            .into();
        let noise_value = self.noise.get(noise_point);
        self.noise_threshold - noise_value as f32
    }
}

impl SDFRotation {
    pub fn from_axis_angle(child_id: SDFNodeID, axis: Vector3<f32>, angle: f32) -> Self {
        let rotation = UnitQuaternion::from_axis_angle(&UnitVector3::new_normalize(axis), angle);
        Self { child_id, rotation }
    }
}

impl SDFScaling {
    pub fn new(child_id: SDFNodeID, scaling: f32) -> Self {
        assert!(scaling > 0.0);
        Self { child_id, scaling }
    }
}

impl SDFUnion {
    pub fn new(child_1_id: SDFNodeID, child_2_id: SDFNodeID, smoothness: f32) -> Self {
        assert!(smoothness >= 0.0);
        Self {
            child_1_id,
            child_2_id,
            smoothness,
        }
    }
}

impl SDFSubtraction {
    pub fn new(child_1_id: SDFNodeID, child_2_id: SDFNodeID, smoothness: f32) -> Self {
        assert!(smoothness >= 0.0);
        Self {
            child_1_id,
            child_2_id,
            smoothness,
        }
    }
}

impl SDFIntersection {
    pub fn new(child_1_id: SDFNodeID, child_2_id: SDFNodeID, smoothness: f32) -> Self {
        assert!(smoothness >= 0.0);
        Self {
            child_1_id,
            child_2_id,
            smoothness,
        }
    }
}

impl MultifractalNoiseSDFModifier {
    pub fn new(
        child_id: SDFNodeID,
        octaves: u32,
        frequency: f32,
        lacunarity: f32,
        persistence: f32,
        amplitude: f32,
        seed: u32,
    ) -> Self {
        let noise = HybridMulti::new(seed)
            .set_octaves(octaves as usize)
            .set_frequency(frequency.into())
            .set_lacunarity(lacunarity.into())
            .set_persistence(persistence.into());
        Self {
            child_id,
            noise,
            amplitude,
        }
    }

    fn compute_signed_distance_perturbation(&self, displacement_from_center: &Vector3<f32>) -> f32 {
        let noise_point: [f64; 3] = displacement_from_center.cast().into();
        self.amplitude * self.noise.get(noise_point) as f32
    }
}

impl MultiscaleSphereSDFModifier {
    pub fn new(
        child_id: SDFNodeID,
        octaves: u32,
        max_scale: f32,
        persistence: f32,
        inflation: f32,
        smoothness: f32,
        seed: u32,
    ) -> Self {
        let frequency = 0.5 / max_scale;

        // Scale inflation and smoothness according to the scale of perturbations
        let inflation = max_scale * inflation;
        let smoothness = max_scale * smoothness;

        Self {
            child_id,
            octaves,
            frequency,
            persistence,
            inflation,
            smoothness,
            seed,
        }
    }

    fn max_scale(&self) -> f32 {
        0.5 / self.frequency
    }

    fn modify_signed_distance(&self, position: &Vector3<f32>, signed_distance: f32) -> f32 {
        /// Rotates with an angle of `2 * pi / golden_ratio` around the axis
        /// `[1, 1, 1]` (to break up the regular grid pattern).
        const ROTATION: UnitQuaternion<f32> = UnitQuaternion::new_unchecked(Quaternion::new(
            -0.3623749, 0.5381091, 0.5381091, 0.5381091,
        ));

        let mut parent_distance = signed_distance;
        let mut position = self.frequency * position;
        let mut scale = 1.0;

        for _ in 0..self.octaves {
            let sphere_grid_distance = scale * self.evaluate_sphere_grid_sdf(&position);

            let intersected_sphere_grid_distance = smooth_sdf_intersection(
                sphere_grid_distance,
                parent_distance - self.inflation * scale,
                self.smoothness * scale,
            );

            parent_distance = smooth_sdf_union(
                intersected_sphere_grid_distance,
                parent_distance,
                self.smoothness * scale,
            );

            position = ROTATION * (position / self.persistence);

            scale *= self.persistence;
        }
        parent_distance
    }

    fn evaluate_sphere_grid_sdf(&self, position: &Vector3<f32>) -> f32 {
        const CORNER_OFFSETS: [Vector3<i32>; 8] = [
            vector![0, 0, 0],
            vector![0, 0, 1],
            vector![0, 1, 0],
            vector![0, 1, 1],
            vector![1, 0, 0],
            vector![1, 0, 1],
            vector![1, 1, 0],
            vector![1, 1, 1],
        ];
        let grid_cell_indices = position.map(|coord| coord.floor() as i32);
        let offset_in_grid_cell = position - grid_cell_indices.cast();

        CORNER_OFFSETS
            .iter()
            .map(|corner_offsets| {
                OrderedFloat(self.evaluate_corner_sphere_sdf(
                    &grid_cell_indices,
                    &offset_in_grid_cell,
                    corner_offsets,
                ))
            })
            .min()
            .unwrap()
            .0
    }

    fn evaluate_corner_sphere_sdf(
        &self,
        grid_cell_indices: &Vector3<i32>,
        offset_in_grid_cell: &Vector3<f32>,
        corner_offsets: &Vector3<i32>,
    ) -> f32 {
        let sphere_radius = self.corner_sphere_radius(grid_cell_indices, corner_offsets);
        let distance_to_sphere_center = (offset_in_grid_cell - corner_offsets.cast()).magnitude();
        distance_to_sphere_center - sphere_radius
    }

    /// Every sphere gets a random radius based on its location in the grid.
    fn corner_sphere_radius(
        &self,
        grid_cell_indices: &Vector3<i32>,
        corner_offsets: &Vector3<i32>,
    ) -> f32 {
        // The maximum radius is half the extent of a grid cell, i.e. 0.5
        const HASH_TO_RADIUS: f32 = 0.5 / u32::MAX as f32;
        let hash = XxHash32::oneshot(
            self.seed,
            bytemuck::bytes_of(&(grid_cell_indices + corner_offsets)),
        );
        HASH_TO_RADIUS * hash as f32
    }
}

impl VoxelTypeGenerator {
    fn voxel_type_at_indices(&self, i: usize, j: usize, k: usize) -> VoxelType {
        match self {
            Self::Same(SameVoxelTypeGenerator { voxel_type }) => *voxel_type,
            Self::GradientNoise(generator) => generator.voxel_type_at_indices(i, j, k),
        }
    }
}

impl From<SameVoxelTypeGenerator> for VoxelTypeGenerator {
    fn from(generator: SameVoxelTypeGenerator) -> Self {
        Self::Same(generator)
    }
}

impl From<GradientNoiseVoxelTypeGenerator> for VoxelTypeGenerator {
    fn from(generator: GradientNoiseVoxelTypeGenerator) -> Self {
        Self::GradientNoise(generator)
    }
}

impl SameVoxelTypeGenerator {
    pub fn new(voxel_type: VoxelType) -> Self {
        Self { voxel_type }
    }
}

impl GradientNoiseVoxelTypeGenerator {
    pub fn new(
        voxel_types: Vec<VoxelType>,
        noise_frequency: f64,
        voxel_type_frequency: f64,
        seed: u32,
    ) -> Self {
        assert!(!voxel_types.is_empty());

        let noise_scale_for_voxel_type_dim = voxel_type_frequency / voxel_types.len() as f64;

        let noise = Simplex::new(seed);

        Self {
            voxel_types,
            noise_frequency,
            noise_scale_for_voxel_type_dim,
            noise,
        }
    }

    fn voxel_type_at_indices(&self, i: usize, j: usize, k: usize) -> VoxelType {
        let x = i as f64 * self.noise_frequency;
        let y = j as f64 * self.noise_frequency;
        let z = k as f64 * self.noise_frequency;

        self.voxel_types
            .iter()
            .enumerate()
            .map(|(voxel_type_idx, voxel_type)| {
                let voxel_type_coord = voxel_type_idx as f64 * self.noise_scale_for_voxel_type_dim;
                let noise_value = self.noise.get([x, y, z, voxel_type_coord]);
                (noise_value, *voxel_type)
            })
            .max_by_key(|(noise_value, _)| OrderedFloat(*noise_value))
            .unwrap()
            .1
    }
}

fn zero_domain() -> AxisAlignedBox<f32> {
    AxisAlignedBox::new(Point3::origin(), Point3::origin())
}

fn pad_domain(domain: AxisAlignedBox<f32>, padding: f32) -> AxisAlignedBox<f32> {
    if padding == 0.0 {
        return domain;
    }
    let padding = Vector3::repeat(padding);
    AxisAlignedBox::new(
        domain.lower_corner() - padding,
        domain.upper_corner() + padding,
    )
}

fn domain_padding_for_smoothness(smoothness: f32) -> f32 {
    0.25 * smoothness
}

fn sdf_union(distance_1: f32, distance_2: f32, smoothness: f32) -> f32 {
    if smoothness == 0.0 {
        f32::min(distance_1, distance_2)
    } else {
        smooth_sdf_union(distance_1, distance_2, smoothness)
    }
}

fn sdf_subtraction(distance_1: f32, distance_2: f32, smoothness: f32) -> f32 {
    if smoothness == 0.0 {
        f32::max(distance_1, -distance_2)
    } else {
        smooth_sdf_subtraction(distance_1, distance_2, smoothness)
    }
}

fn sdf_intersection(distance_1: f32, distance_2: f32, smoothness: f32) -> f32 {
    if smoothness == 0.0 {
        f32::max(distance_1, distance_2)
    } else {
        smooth_sdf_intersection(distance_1, distance_2, smoothness)
    }
}

fn smooth_sdf_union(distance_1: f32, distance_2: f32, smoothness: f32) -> f32 {
    let h = (0.5 + 0.5 * (distance_2 - distance_1) / smoothness).clamp(0.0, 1.0);
    mix(distance_2, distance_1, h) - smoothness * h * (1.0 - h)
}

fn smooth_sdf_subtraction(distance_1: f32, distance_2: f32, smoothness: f32) -> f32 {
    let h = (0.5 - 0.5 * (distance_2 + distance_1) / smoothness).clamp(0.0, 1.0);
    mix(distance_2, -distance_1, h) + smoothness * h * (1.0 - h)
}

fn smooth_sdf_intersection(distance_1: f32, distance_2: f32, smoothness: f32) -> f32 {
    let h = (0.5 - 0.5 * (distance_2 - distance_1) / smoothness).clamp(0.0, 1.0);
    mix(distance_2, distance_1, h) + smoothness * h * (1.0 - h)
}

fn mix(a: f32, b: f32, factor: f32) -> f32 {
    (1.0 - factor) * a + factor * b
}

#[cfg(feature = "fuzzing")]
pub mod fuzzing {
    use super::*;
    use crate::voxel_types::VoxelTypeRegistry;
    use allocator_api2::alloc::Global;
    use arbitrary::{Arbitrary, MaxRecursionReached, Result, Unstructured, size_hint};
    use std::mem;

    const MAX_SIZE: usize = 200;

    #[allow(clippy::large_enum_variant)]
    #[derive(Clone, Debug, Arbitrary)]
    enum ArbitrarySDFGeneratorNode {
        Box(BoxSDFGenerator),
        Sphere(SphereSDFGenerator),
        GradientNoise(GradientNoiseSDFGenerator),
    }

    impl<'a> Arbitrary<'a> for SDFVoxelGenerator {
        fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
            let voxel_extent = 10.0 * arbitrary_norm_f64(u)?.max(1e-6);
            let sdf_generator = u.arbitrary()?;
            let voxel_type_generator = u.arbitrary()?;
            Ok(Self::new(voxel_extent, sdf_generator, voxel_type_generator))
        }

        fn size_hint(depth: usize) -> (usize, Option<usize>) {
            Self::try_size_hint(depth).unwrap_or_default()
        }

        fn try_size_hint(depth: usize) -> Result<(usize, Option<usize>), MaxRecursionReached> {
            size_hint::try_recursion_guard(depth, |depth| {
                Ok(size_hint::and_all(&[
                    (mem::size_of::<i32>(), Some(mem::size_of::<i32>())),
                    SDFGenerator::size_hint(depth),
                    VoxelTypeGenerator::size_hint(depth),
                ]))
            })
        }
    }

    impl Arbitrary<'_> for SDFGenerator {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let primitive = match u.arbitrary()? {
                ArbitrarySDFGeneratorNode::Box(generator) => SDFGeneratorNode::Box(generator),
                ArbitrarySDFGeneratorNode::Sphere(generator) => SDFGeneratorNode::Sphere(generator),
                ArbitrarySDFGeneratorNode::GradientNoise(generator) => {
                    SDFGeneratorNode::GradientNoise(generator)
                }
            };
            let mut nodes = AVec::new();
            nodes.push(primitive);
            Ok(Self::new(Global, nodes, 0).unwrap())
        }

        fn size_hint(depth: usize) -> (usize, Option<usize>) {
            ArbitrarySDFGeneratorNode::size_hint(depth)
        }
    }

    impl Arbitrary<'_> for BoxSDFGenerator {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let extent_x =
                u.arbitrary_len::<usize>()?.clamp(1, MAX_SIZE - 1) as f32 + arbitrary_norm_f32(u)?;
            let extent_y =
                u.arbitrary_len::<usize>()?.clamp(1, MAX_SIZE - 1) as f32 + arbitrary_norm_f32(u)?;
            let extent_z =
                u.arbitrary_len::<usize>()?.clamp(1, MAX_SIZE - 1) as f32 + arbitrary_norm_f32(u)?;
            Ok(Self::new([extent_x, extent_y, extent_z]))
        }

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            let size = 6 * mem::size_of::<usize>();
            (size, Some(size))
        }
    }

    impl Arbitrary<'_> for SphereSDFGenerator {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let radius = u.arbitrary_len::<usize>()?.clamp(1, MAX_SIZE / 2 - 1) as f32
                + arbitrary_norm_f32(u)?;
            Ok(Self::new(radius))
        }

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            let size = 2 * mem::size_of::<usize>();
            (size, Some(size))
        }
    }

    impl Arbitrary<'_> for GradientNoiseSDFGenerator {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let extent_x =
                u.arbitrary_len::<usize>()?.clamp(1, MAX_SIZE - 1) as f32 + arbitrary_norm_f32(u)?;
            let extent_y =
                u.arbitrary_len::<usize>()?.clamp(1, MAX_SIZE - 1) as f32 + arbitrary_norm_f32(u)?;
            let extent_z =
                u.arbitrary_len::<usize>()?.clamp(1, MAX_SIZE - 1) as f32 + arbitrary_norm_f32(u)?;
            let noise_frequency = 0.15 * arbitrary_norm_f32(u)?;
            let noise_threshold = arbitrary_norm_f32(u)?;
            let seed = u.arbitrary()?;
            Ok(Self::new(
                [extent_x, extent_y, extent_z],
                noise_frequency,
                noise_threshold,
                seed,
            ))
        }

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            let size = 8 * mem::size_of::<usize>() + mem::size_of::<u32>();
            (size, Some(size))
        }
    }

    impl Arbitrary<'_> for SameVoxelTypeGenerator {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let idx = u.arbitrary()?;
            Ok(Self::new(VoxelType::from_idx_u8(idx)))
        }

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            let size = mem::size_of::<u8>();
            (size, Some(size))
        }
    }

    impl Arbitrary<'_> for GradientNoiseVoxelTypeGenerator {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let mut voxel_types: Vec<_> = (0..VoxelTypeRegistry::max_n_voxel_types())
                .map(VoxelType::from_idx)
                .collect();
            for _ in 0..u.int_in_range(0..=voxel_types.len() - 1)? {
                voxel_types.swap_remove(u.int_in_range(0..=voxel_types.len() - 1)?);
            }
            let noise_frequency = 0.15 * arbitrary_norm_f64(u)?;
            let voxel_type_frequency = 0.15 * arbitrary_norm_f64(u)?;
            let seed = u.arbitrary()?;
            Ok(Self::new(
                voxel_types,
                noise_frequency,
                voxel_type_frequency,
                seed,
            ))
        }

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            let lower_size = mem::size_of::<usize>() + 2 * mem::size_of::<i32>();
            let upper_size =
                lower_size + mem::size_of::<usize>() * (VoxelTypeRegistry::max_n_voxel_types() - 1);
            (lower_size, Some(upper_size))
        }
    }

    fn arbitrary_norm_f64(u: &mut Unstructured<'_>) -> Result<f64> {
        Ok(f64::from(u.int_in_range(0..=1000000)?) / 1000000.0)
    }

    fn arbitrary_norm_f32(u: &mut Unstructured<'_>) -> Result<f32> {
        arbitrary_norm_f64(u).map(|value| value as f32)
    }
}
