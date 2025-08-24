//! Simulation of physics.

#[macro_use]
mod macros;

pub mod anchor;
pub mod collision;
pub mod constraint;
pub mod driven_motion;
pub mod force;
pub mod inertia;
pub mod material;
pub mod medium;
pub mod quantities;
pub mod rigid_body;

#[cfg(feature = "ecs")]
pub mod systems;

use anchor::AnchorManager;
use collision::{Collidable, CollisionWorld};
use constraint::ConstraintManager;
use driven_motion::MotionDriverManager;
use force::ForceGeneratorManager;
use medium::UniformMedium;
use rigid_body::RigidBodyManager;

/// Floating point type used for physics simulation.
#[allow(non_camel_case_types)]
pub type fph = f64;

/// Advances the physics simulation by one time step.
pub fn perform_physics_step<C: Collidable>(
    rigid_body_manager: &mut RigidBodyManager,
    anchor_manager: &AnchorManager,
    force_generator_manager: &ForceGeneratorManager,
    motion_driver_manager: &MotionDriverManager,
    constraint_manager: &mut ConstraintManager,
    collision_world: &mut CollisionWorld<C>,
    collidable_context: &C::Context,
    medium: &UniformMedium,
    current_simulation_time: fph,
    step_duration: fph,
) {
    let new_simulation_time = current_simulation_time + step_duration;

    collision_world.synchronize_collidables_with_rigid_bodies(rigid_body_manager);

    if constraint_manager.solver().config().enabled {
        impact_log::with_timing_info_logging!("Preparing constraints"; {
            constraint_manager.prepare_constraints(
                rigid_body_manager,
                anchor_manager,
                collision_world,
                collidable_context,
            );
        });
    }

    rigid_body_manager.advance_dynamic_rigid_body_momenta(step_duration);

    if constraint_manager.solver().config().enabled {
        impact_log::with_timing_info_logging!("Solving constraints"; {
            constraint_manager.compute_and_apply_constrained_state(rigid_body_manager);
        });
    }

    rigid_body_manager.advance_dynamic_rigid_body_configurations(step_duration);

    // We really only want to advance non-driven kinematic bodies, but since
    // the bodies with a motion driver will have their state overwritten
    // when we call `apply_motion` anyway, we advance all kinematic bodies
    // here for simplicity
    rigid_body_manager.advance_kinematic_rigid_body_configurations(step_duration);

    motion_driver_manager.apply_motion(rigid_body_manager, new_simulation_time);

    force_generator_manager.apply_forces_and_torques(medium, rigid_body_manager, anchor_manager);
}
