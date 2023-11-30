//! Tasks representing ECS systems related to rigid bodies.

use crate::{
    define_task,
    physics::{
        AngularVelocityComp, OrientationComp, PhysicsTag, PositionComp, RigidBodyComp, VelocityComp,
    },
    world::World,
};
use impact_ecs::query;

define_task!(
    /// This [`Task`](crate::scheduling::Task) updates the position,
    /// orientation, velocity and angular velocity of each rigid body entity to
    /// make them in sync with the current state of the entity's
    /// [`RigidBody`](crate::physics::RigidBody).
    [pub] SyncRigidBodyMotion,
    depends_on = [],
    execute_on = [PhysicsTag],
    |world: &World| {
        with_debug_logging!("Synchronizing rigid body motion"; {
            let simulator = world.simulator().read().unwrap();
            let rigid_body_manager = simulator.rigid_body_manager().read().unwrap();
            let ecs_world = world.ecs_world().read().unwrap();
            query!(
                ecs_world,
                |position: &mut PositionComp,
                 orientation: &mut OrientationComp,
                 velocity: &mut VelocityComp,
                 angular_velocity: &mut AngularVelocityComp,
                 rigid_body_comp: &RigidBodyComp| {
                    let rigid_body = rigid_body_manager.rigid_body(rigid_body_comp.id);
                    position.0 = rigid_body.compute_position();
                    orientation.0 = *rigid_body.orientation();
                    velocity.0 = *rigid_body.velocity();
                    angular_velocity.0 = *rigid_body.angular_velocity();
                }
            );
            Ok(())
        })
    }
);
