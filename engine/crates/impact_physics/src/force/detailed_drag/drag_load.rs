//! Calculation of forces and torques due to drag.

use crate::quantities::{Direction, DirectionP, Force, Orientation, Position, PositionP, Torque};
use impact_alloc::{AVec, Allocator};
use impact_math::{
    Float,
    point::Point3P,
    vector::{UnitVector3, UnitVector3P, Vector3P},
};
use std::ops::{Add, AddAssign, Div, Mul};

/// A load (force and torque) due to drag.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default)]
pub struct DragLoad {
    /// The drag force on the center of mass.
    pub force: Vector3P,
    /// The drag torque around the center of mass.
    pub torque: Vector3P,
}

/// Helper struct for accumulating drag loads and averaging them.
#[derive(Clone, Debug, Default)]
pub struct AveragingDragLoad {
    summed_load: DragLoad,
    summed_weight: f32,
}

/// The properties of a mesh triangle relevant for computign the drag force on
/// it.
#[derive(Clone, Debug)]
pub struct MeshTriangleDragProperties {
    center: PositionP,
    normal_vector: UnitVector3P,
    area: f32,
}

impl DragLoad {
    /// Computes the total drag force and torque in world space for a body with
    /// this drag load, given the scale factor for its mesh, its drag
    /// coefficient and orientation, and the square of its speed relative to the
    /// medium.
    pub fn compute_world_space_drag_force_and_torque(
        &self,
        mesh_scaling: f32,
        medium_mass_density: f32,
        drag_coefficient: f32,
        body_orientation: &Orientation,
        squared_body_speed_relative_to_medium: f32,
    ) -> (Force, Torque) {
        let force = self.force.unpack();
        let torque = self.torque.unpack();

        // The force is proportional to mesh area, medium mass density, drag
        // coefficient and squared speed
        let force_scaling = mesh_scaling.powi(2)
            * medium_mass_density
            * drag_coefficient
            * squared_body_speed_relative_to_medium;
        // The torque is proportional to the force and additionally scales with
        // the mesh extent (distance from center of mass)
        let torque_scaling = mesh_scaling * force_scaling;

        let world_space_force = force_scaling * body_orientation.rotate_vector(&force);
        let world_space_torque = torque_scaling * body_orientation.rotate_vector(&torque);

        (world_space_force, world_space_torque)
    }
}

impl Add for &DragLoad {
    type Output = DragLoad;

    fn add(self, rhs: Self) -> Self::Output {
        DragLoad {
            force: self.force + rhs.force,
            torque: self.torque + rhs.torque,
        }
    }
}

impl AddAssign for DragLoad {
    fn add_assign(&mut self, rhs: Self) {
        *self = self.add(&rhs);
    }
}

impl Mul<f32> for &DragLoad {
    type Output = DragLoad;

    fn mul(self, factor: f32) -> Self::Output {
        DragLoad {
            force: self.force * factor,
            torque: self.torque * factor,
        }
    }
}

impl Div<f32> for &DragLoad {
    type Output = DragLoad;

    fn div(self, divisor: f32) -> Self::Output {
        DragLoad {
            force: self.force / divisor,
            torque: self.torque / divisor,
        }
    }
}

impl AveragingDragLoad {
    /// Adds the given drag load for averaging with the given weight. All
    /// weights need not sum up to one.
    pub fn add_sample(&mut self, load: &DragLoad, weight: f32) {
        self.summed_load += load * weight;
        self.summed_weight += weight;
    }

    /// Evaluates the weighted average of the drag loads accumulated by calling
    /// [`add_sample`](Self::add_sample). The returned average drag load can be
    /// cast to a different floating point type.
    pub fn into_average_load(self) -> DragLoad {
        let Self {
            summed_load,
            summed_weight,
        } = self;

        let average_load = if summed_weight > 0.0 {
            (&summed_load) / summed_weight
        } else {
            summed_load
        };

        average_load
    }
}

/// Computes the total drag force on the center of mass and torque around the
/// center of mass (excluding triangle-invariant factors such as relative speed
/// of the medium) for the given mesh triangles for the given number of
/// directions. The sampled directions will be close to uniformly distributed.
///
/// # Returns
/// A [`Vec`] with each pair of direction (against the relative flow of the
/// medium) and aggregate drag load.
pub fn compute_aggregate_drag_loads_for_uniformly_distributed_directions<'a, A: Allocator>(
    alloc: A,
    triangle_vertex_positions: impl IntoIterator<Item = [&'a Point3P; 3]>,
    center_of_mass: &Position,
    n_direction_samples: usize,
) -> AVec<(DirectionP, DragLoad), A> {
    let mesh_triangles = compute_mesh_triangle_drag_properties(alloc, triangle_vertex_positions);

    let mut drag_loads = AVec::with_capacity_in(n_direction_samples, alloc);

    drag_loads.extend(
        impact_geometry::compute_uniformly_distributed_radial_directions(n_direction_samples).map(
            |direction| {
                let load = compute_aggregate_drag_load_for_direction(
                    &mesh_triangles,
                    center_of_mass,
                    &direction,
                );
                (direction.pack(), load)
            },
        ),
    );

    drag_loads
}

/// Computes the total drag force on the center of mass and torque around the
/// center of mass (excluding triangle-invariant factors such as the relative
/// speed of the medium) for the given mesh when the mesh is moving in the given
/// direction relative to the medium.
pub fn compute_aggregate_drag_load_for_direction(
    mesh_triangles: &[MeshTriangleDragProperties],
    center_of_mass: &Position,
    direction_against_flow: &Direction,
) -> DragLoad {
    let mut total_force = Force::zeros();
    let mut total_torque = Torque::zeros();

    for triangle in mesh_triangles {
        let center = triangle.center.unpack();
        let normal_vector = triangle.normal_vector.unpack();

        let cos_impact_angle = direction_against_flow.dot(&normal_vector);

        // Only triangles facing the incoming flow are affected
        if cos_impact_angle > 0.0 {
            // The drag force is proportional to the projected area
            let projected_area = cos_impact_angle * triangle.area;
            // let projected_area = triangle.area;
            // It is directed inwards perpendicularly to the surface
            let drag_force = (-projected_area) * normal_vector;
            // If the line of force does not go through the center of mass,
            // there is also an associated torque
            let drag_torque = (center - center_of_mass).cross(&drag_force);

            total_force += drag_force;
            total_torque += drag_torque;
        }
    }

    DragLoad {
        force: total_force.pack(),
        torque: total_torque.pack(),
    }
}

/// Computes the properties required for calculating drag for each
/// non-degenerate triangle in the given iterator and returns them in an array.
pub fn compute_mesh_triangle_drag_properties<'a, A: Allocator>(
    alloc: A,
    triangle_vertex_positions: impl IntoIterator<Item = [&'a Point3P; 3]>,
) -> AVec<MeshTriangleDragProperties, A> {
    let triangle_vertex_positions = triangle_vertex_positions.into_iter();

    let mut props = AVec::with_capacity_in(triangle_vertex_positions.size_hint().0, alloc);

    props.extend(triangle_vertex_positions.into_iter().filter_map(
        |[vertex_1, vertex_2, vertex_3]| {
            let vertex_1 = vertex_1.unpack();
            let vertex_2 = vertex_2.unpack();
            let vertex_3 = vertex_3.unpack();

            let edge_1 = vertex_2 - vertex_1;
            let edge_2 = vertex_3 - vertex_1;

            UnitVector3::normalized_from_and_norm_if_above(edge_1.cross(&edge_2), f32::EPSILON).map(
                |(normal_vector, twice_area)| {
                    let center =
                        f32::ONE_THIRD * (vertex_1 + vertex_2.as_vector() + vertex_3.as_vector());

                    MeshTriangleDragProperties {
                        center: center.pack(),
                        normal_vector: normal_vector.pack(),
                        area: 0.5 * twice_area,
                    }
                },
            )
        },
    ));

    props
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::abs_diff_eq;
    use impact_alloc::Global;
    use impact_math::vector::Vector3;
    use impact_mesh::TriangleMesh;
    use proptest::prelude::*;

    prop_compose! {
        fn direction_strategy()(
            phi in 0.0..f32::TWO_PI,
            theta in 0.0..f32::PI,
        ) -> Direction {
            Direction::normalized_from(Vector3::new(
                f32::cos(phi) * f32::sin(theta),
                f32::sin(phi) * f32::sin(theta),
                f32::cos(theta)
            ))
        }
    }

    #[cfg(not(miri))]
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10))]
        #[test]
        fn should_compute_correct_drag_load_for_sphere(
            direction in direction_strategy()
        ) {
            let sphere_mesh = TriangleMesh::create_sphere(40);
            let triangle_drag_properties = compute_mesh_triangle_drag_properties(Global, sphere_mesh.triangle_vertex_positions());

            let load = compute_aggregate_drag_load_for_direction(
                &triangle_drag_properties,
                &Position::origin(),
                &direction,
            );

            let (force_direction, force) = UnitVector3::normalized_from_and_norm(load.force.unpack());

            // In the accumulation of forces on the individual triangles over
            // the front-facing hemisphere, the components perpendicular to the
            // medium's flow direction will cancel due to symmetry. The
            // remaining component scales as cos(theta). Another cos(theta)
            // factor comes from using the projected triangle area. So the force
            // accumulation corresponds to integrating cos(theta)^2 over the
            // hemisphere, giving (2/3)*pi*r^2.
            let correct_force = f32::ONE_THIRD * f32::TWO_PI * 0.5_f32.powi(2);

            prop_assert!(abs_diff_eq!(force_direction, -direction, epsilon = 1e-3));
            prop_assert!(abs_diff_eq!(force, correct_force, epsilon = 1e-3));
            prop_assert!(abs_diff_eq!(load.torque, Vector3P::zeros(), epsilon = 1e-3));
        }
    }
}
