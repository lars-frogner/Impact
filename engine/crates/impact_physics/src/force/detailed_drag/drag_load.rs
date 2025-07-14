//! Calculation of forces and torques due to drag.

use crate::{
    fph,
    quantities::{Direction, Force, Orientation, Position, Torque},
};
use anyhow::Result;
use impact_math::{Angle, Float, Radians};
use nalgebra::{Point3, UnitVector3, Vector3, vector};
use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned, ser::Serializer};
use simba::scalar::SubsetOf;
use std::ops::{Add, AddAssign, Div, Mul};

/// A load (force and torque) due to drag.
#[derive(Clone, Debug, Default)]
pub struct DragLoad<F: Float> {
    /// The drag force on the center of mass.
    pub force: Vector3<F>,
    /// The drag torque around the center of mass.
    pub torque: Vector3<F>,
}

/// Helper struct for accumulating drag loads and averaging them.
#[derive(Clone, Debug, Default)]
pub struct AveragingDragLoad<F: Float> {
    summed_load: DragLoad<F>,
    summed_weight: F,
}

/// The properties of a mesh triangle relevant for computign the drag force on
/// it.
#[derive(Clone, Debug)]
pub struct MeshTriangleDragProperties {
    center: Position,
    normal_vector: UnitVector3<fph>,
    area: fph,
}

impl<F: Float> DragLoad<F> {
    /// Computes the total drag force and torque in world space for a body with
    /// this drag load, given the scale factor for its mesh, its drag
    /// coefficient and orientation, and the square of its speed relative to the
    /// medium.
    pub fn compute_world_space_drag_force_and_torque(
        &self,
        mesh_scaling: fph,
        medium_mass_density: fph,
        drag_coefficient: fph,
        body_orientation: &Orientation,
        squared_body_speed_relative_to_medium: fph,
    ) -> (Force, Torque)
    where
        F: SubsetOf<fph>,
    {
        // The force is proportional to mesh area, medium mass density, drag
        // coefficient and squared speed
        let force_scaling = mesh_scaling.powi(2)
            * medium_mass_density
            * drag_coefficient
            * squared_body_speed_relative_to_medium;
        // The torque is proportional to the force and additionally scales with
        // the mesh extent (distance from center of mass)
        let torque_scaling = mesh_scaling * force_scaling;

        let world_space_force =
            force_scaling * body_orientation.transform_vector(&self.force.cast::<fph>());
        let world_space_torque =
            torque_scaling * body_orientation.transform_vector(&self.torque.cast::<fph>());

        (world_space_force, world_space_torque)
    }
}

impl<F: Float> Add for &DragLoad<F> {
    type Output = DragLoad<F>;

    fn add(self, rhs: Self) -> Self::Output {
        DragLoad {
            force: self.force + rhs.force,
            torque: self.torque + rhs.torque,
        }
    }
}

impl<F: Float> AddAssign for DragLoad<F> {
    fn add_assign(&mut self, rhs: Self) {
        *self = self.add(&rhs);
    }
}

impl<F: Float> Mul<F> for &DragLoad<F> {
    type Output = DragLoad<F>;

    fn mul(self, factor: F) -> Self::Output {
        DragLoad {
            force: self.force * factor,
            torque: self.torque * factor,
        }
    }
}

impl<F: Float> Div<F> for &DragLoad<F> {
    type Output = DragLoad<F>;

    fn div(self, divisor: F) -> Self::Output {
        let inv_divisor = F::ONE / divisor;
        self * inv_divisor
    }
}

impl<F: Float + Serialize + DeserializeOwned> Serialize for DragLoad<F> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (
            &(self.force.x, self.force.y, self.force.z),
            &(self.torque.x, self.torque.y, self.torque.z),
        )
            .serialize(serializer)
    }
}

impl<'de, F: Float + Deserialize<'de>> Deserialize<'de> for DragLoad<F> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let ((force_x, force_y, force_z), (torque_x, torque_y, torque_z)) =
            <((F, F, F), (F, F, F))>::deserialize(deserializer)?;

        Ok(DragLoad {
            force: vector![force_x, force_y, force_z],
            torque: vector![torque_x, torque_y, torque_z],
        })
    }
}

impl<F: Float> AveragingDragLoad<F> {
    /// Adds the given drag load for averaging with the given weight. All
    /// weights need not sum up to one.
    pub fn add_sample(&mut self, load: &DragLoad<F>, weight: F) {
        self.summed_load += load * weight;
        self.summed_weight += weight;
    }

    /// Evaluates the weighted average of the drag loads accumulated by calling
    /// [`add_sample`](Self::add_sample). The returned average drag load can be
    /// cast to a different floating point type.
    pub fn into_average_load<FNEW>(self) -> DragLoad<FNEW>
    where
        F: SubsetOf<FNEW>,
        FNEW: Float,
    {
        let Self {
            summed_load,
            summed_weight,
        } = self;

        let average_load = if summed_weight > F::ZERO {
            (&summed_load) / summed_weight
        } else {
            summed_load
        };

        let DragLoad { force, torque } = average_load;

        DragLoad {
            force: force.cast::<FNEW>(),
            torque: torque.cast::<FNEW>(),
        }
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
pub fn compute_aggregate_drag_loads_for_uniformly_distributed_directions<'a, F>(
    triangle_vertex_positions: impl IntoIterator<Item = [&'a Point3<F>; 3]>,
    center_of_mass: &Position,
    n_direction_samples: usize,
) -> Vec<(Direction, DragLoad<fph>)>
where
    F: Float + SubsetOf<fph>,
{
    let mesh_triangles = compute_mesh_triangle_drag_properties(triangle_vertex_positions);

    compute_uniformly_distributed_directions(n_direction_samples)
        .map(|direction| {
            let load = compute_aggregate_drag_load_for_direction(
                &mesh_triangles,
                center_of_mass,
                &direction,
            );
            (direction, load)
        })
        .collect()
}

/// Computes the total drag force on the center of mass and torque around the
/// center of mass (excluding triangle-invariant factors such as the relative
/// speed of the medium) for the given mesh when the mesh is moving in the given
/// direction relative to the medium.
pub fn compute_aggregate_drag_load_for_direction(
    mesh_triangles: &[MeshTriangleDragProperties],
    center_of_mass: &Position,
    direction_against_flow: &Direction,
) -> DragLoad<fph> {
    let mut total_force = Force::zeros();
    let mut total_torque = Torque::zeros();

    for triangle in mesh_triangles {
        let cos_impact_angle = direction_against_flow.dot(&triangle.normal_vector);

        // Only triangles facing the incoming flow are affected
        if cos_impact_angle > 0.0 {
            // The drag force is proportional to the projected area
            let projected_area = cos_impact_angle * triangle.area;
            // let projected_area = triangle.area;
            // It is directed inwards perpendicularly to the surface
            let drag_force = (-projected_area) * triangle.normal_vector.as_ref();
            // If the line of force does not go through the center of mass,
            // there is also an associated torque
            let drag_torque = (triangle.center - center_of_mass).cross(&drag_force);

            total_force += drag_force;
            total_torque += drag_torque;
        }
    }

    DragLoad {
        force: total_force,
        torque: total_torque,
    }
}

/// Computes the properties required for calculating drag for each
/// non-degenerate triangle in the given iterator and returns them in an array.
pub fn compute_mesh_triangle_drag_properties<'a, F>(
    triangle_vertex_positions: impl IntoIterator<Item = [&'a Point3<F>; 3]>,
) -> Vec<MeshTriangleDragProperties>
where
    F: Float + SubsetOf<fph>,
{
    triangle_vertex_positions
        .into_iter()
        .filter_map(|[vertex_1, vertex_2, vertex_3]| {
            let vertex_1 = vertex_1.cast::<fph>();
            let vertex_2 = vertex_2.cast::<fph>();
            let vertex_3 = vertex_3.cast::<fph>();

            let edge_1 = vertex_2 - vertex_1;
            let edge_2 = vertex_3 - vertex_1;

            UnitVector3::try_new_and_get(edge_1.cross(&edge_2), fph::EPSILON).map(
                |(normal_vector, twice_area)| {
                    let center = fph::ONE_THIRD * (vertex_1 + vertex_2.coords + vertex_3.coords);
                    MeshTriangleDragProperties {
                        center,
                        normal_vector,
                        area: 0.5 * twice_area,
                    }
                },
            )
        })
        .collect()
}

/// Computes the given number of directions, making them close to uniformly
/// distributed.
///
/// # Returns
/// An iterator over the directions.
///
/// # Panics
/// If the given number of directions is zero.
pub fn compute_uniformly_distributed_directions(
    n_direction_samples: usize,
) -> impl Iterator<Item = Direction> {
    assert_ne!(n_direction_samples, 0);

    let idx_norm = 1.0 / ((n_direction_samples - 1) as fph);
    let golden_angle = compute_golden_angle();

    (0..n_direction_samples).map(move |idx| {
        // Distribute evenly in z
        let z = 1.0 - 2.0 * (idx as fph) * idx_norm;
        let horizontal_radius = fph::sqrt(1.0 - z.powi(2));

        // Use golden angle to space the azimuthal angles, giving a close to
        // uniform distribution over the sphere
        let azimuthal_angle = (idx as fph) * golden_angle.radians();

        let (sin_azimuthal_angle, cos_azimuthal_angle) = azimuthal_angle.sin_cos();
        let x = horizontal_radius * cos_azimuthal_angle;
        let y = horizontal_radius * sin_azimuthal_angle;

        Direction::new_normalize(vector![x, y, z])
    })
}

fn compute_golden_angle() -> Radians<fph> {
    Radians(fph::PI * (3.0 - fph::sqrt(5.0)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::abs_diff_eq;
    use impact_mesh::triangle::TriangleMesh;
    use proptest::prelude::*;

    prop_compose! {
        fn direction_strategy()(
            phi in 0.0..fph::TWO_PI,
            theta in 0.0..fph::PI,
        ) -> Direction {
            Direction::new_normalize(vector![
                fph::cos(phi) * fph::sin(theta),
                fph::sin(phi) * fph::sin(theta),
                fph::cos(theta)
            ])
        }
    }

    #[cfg(not(miri))]
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10))]
        #[test]
        fn should_compute_correct_drag_load_for_sphere(
            direction in direction_strategy()
        ) {
            let sphere_mesh = TriangleMesh::<f32>::create_sphere(40);
            let triangle_drag_properties = compute_mesh_triangle_drag_properties(sphere_mesh.triangle_vertex_positions());

            let load = compute_aggregate_drag_load_for_direction(
                &triangle_drag_properties,
                &Position::origin(),
                &direction,
            );

            let (force_direction, force) = UnitVector3::new_and_get(load.force);

            // In the accumulation of forces on the individual triangles over
            // the front-facing hemisphere, the components perpendicular to the
            // medium's flow direction will cancel due to symmetry. The
            // remaining component scales as cos(theta). Another cos(theta)
            // factor comes from using the projected triangle area. So the force
            // accumulation corresponds to integrating cos(theta)^2 over the
            // hemisphere, giving (2/3)*pi*r^2.
            let correct_force = fph::ONE_THIRD * fph::TWO_PI * (0.5_f64).powi(2);

            prop_assert!(abs_diff_eq!(force_direction, -direction, epsilon = 1e-3));
            prop_assert!(abs_diff_eq!(force, correct_force, epsilon = 1e-3));
            prop_assert!(abs_diff_eq!(load.torque, Vector3::zeros(), epsilon = 1e-3));
        }
    }
}
