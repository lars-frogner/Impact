//! Spring force.

pub mod components;
pub mod systems;

use crate::physics::{
    fph,
    motion::{Direction, Orientation, Position},
};
use bytemuck::{Pod, Zeroable};
use nalgebra::Vector3;

/// A spring or elastic band.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Spring {
    /// The spring constant representing the stiffness of the spring.
    pub stiffness: fph,
    /// The spring damping coefficient.
    pub damping: fph,
    /// The length for which the spring is in equilibrium.
    pub rest_length: fph,
    /// The length below which the spring force is always zero.
    pub slack_length: fph,
}

/// The current state of a spring.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct SpringState {
    /// The direction from the first to the second attachment point.
    direction: Direction,
    /// The position of the center of the spring.
    center: Position,
}

impl Spring {
    /// Creates a new spring.
    pub fn new(stiffness: fph, damping: fph, rest_length: fph, slack_length: fph) -> Self {
        Self {
            stiffness,
            damping,
            rest_length,
            slack_length,
        }
    }

    /// Creates a standard spring (no slack).
    pub fn standard(stiffness: fph, damping: fph, rest_length: fph) -> Self {
        Self::new(stiffness, damping, rest_length, 0.0)
    }

    /// Creates an elastic band that is slack below a given length.
    pub fn elastic_band(stiffness: fph, damping: fph, slack_length: fph) -> Self {
        Self::new(stiffness, damping, slack_length, slack_length)
    }

    /// Computes the force along the spring axis for the given length and rate
    /// of change in length. A positive force is directed outward.
    pub fn scalar_force(&self, length: fph, rate_of_length_change: fph) -> fph {
        if length <= self.slack_length {
            0.0
        } else {
            self.compute_spring_force(length) + self.compute_damping_force(rate_of_length_change)
        }
    }

    fn compute_spring_force(&self, length: fph) -> fph {
        -self.stiffness * (length - self.rest_length)
    }

    fn compute_damping_force(&self, rate_of_length_change: fph) -> fph {
        -self.damping * rate_of_length_change
    }
}

impl SpringState {
    /// Creates a new spring state (with dummy values).
    pub fn new() -> Self {
        Self {
            direction: Vector3::y_axis(),
            center: Position::origin(),
        }
    }

    /// Returns the direction from the first to the second attachment point.
    pub fn direction(&self) -> &Direction {
        &self.direction
    }

    /// Returns the position of the center of the spring.
    pub fn center(&self) -> &Position {
        &self.center
    }

    /// Computes an orientation for the spring based on its direction.
    pub fn compute_orientation(&self) -> Orientation {
        Orientation::rotation_between_axis(&Vector3::y_axis(), &self.direction)
            .unwrap_or_else(|| Orientation::from_axis_angle(&Vector3::y_axis(), 0.0))
    }

    fn update(&mut self, attachment_point_1: &Position, direction: Direction, length: fph) {
        self.center = attachment_point_1 + direction.as_ref() * (0.5 * length);
        self.direction = direction;
    }

    fn update_with_zero_length(&mut self, attachment_point_1: Position) {
        self.center = attachment_point_1;
    }
}

impl Default for SpringState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn should_get_zero_undamped_force_at_rest_length() {
        let rest_length = 1.0;
        let spring = Spring::standard(1.0, 0.0, rest_length);
        assert_abs_diff_eq!(spring.scalar_force(rest_length, 0.0), 0.0);
    }

    #[test]
    fn should_get_positive_undamped_force_below_rest_length() {
        let rest_length = 1.0;
        let spring = Spring::standard(1.0, 0.0, rest_length);
        assert!(spring.scalar_force(0.5 * rest_length, 0.0) > 0.0);
    }

    #[test]
    fn should_get_negative_undamped_force_above_rest_length() {
        let rest_length = 1.0;
        let spring = Spring::standard(1.0, 0.0, rest_length);
        assert!(spring.scalar_force(2.0 * rest_length, 0.0) < 0.0);
    }

    #[test]
    fn should_get_zero_force_below_slack_length() {
        let slack_length = 1.0;
        let spring = Spring::elastic_band(1.0, 1.0, slack_length);
        assert_abs_diff_eq!(spring.scalar_force(0.5 * slack_length, -1.0), 0.0);
    }

    #[test]
    fn should_get_positive_damping_force_for_contracting_spring() {
        let rest_length = 1.0;
        let spring = Spring::standard(1.0, 1.0, rest_length);
        assert!(spring.scalar_force(rest_length, -1.0) > 0.0);
    }

    #[test]
    fn should_get_negative_damping_force_for_expanding_spring() {
        let rest_length = 1.0;
        let spring = Spring::standard(1.0, 1.0, rest_length);
        assert!(spring.scalar_force(rest_length, 1.0) < 0.0);
    }
}
