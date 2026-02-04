//! Player info overlay.

use crate::{Game, entities::player::Player};
use anyhow::Result;
use impact::impact_physics::{
    force::alignment_torque::AlignmentDirection, quantities::AccelerationC,
};

#[derive(Debug)]
pub(crate) struct DisplayedPlayerState {
    pub alignment_direction: &'static str,
    pub acceleration: AccelerationC,
    pub inventory_mass: f32,
    pub launch_speed: f32,
}

impl DisplayedPlayerState {
    pub(crate) fn gather(game: &Game) -> Result<Self> {
        let alignment_direction =
            Player::with_alignment_torque_generator(game.engine(), |generator| {
                Ok(generator.alignment_direction.clone())
            })?;

        let (force, mass) = Player::with_rigid_body(game.engine(), |rigid_body| {
            Ok((*rigid_body.total_force(), rigid_body.mass()))
        })?;

        let alignment_direction = match alignment_direction {
            AlignmentDirection::GravityForce => "gravity",
            AlignmentDirection::Fixed(_) => "ecliptic pole",
        };

        let acceleration = force / mass;

        let inventory_mass = game.player.inventory.mass();
        let launch_speed = game.player.launcher.launch_speed();

        Ok(Self {
            alignment_direction,
            acceleration,
            inventory_mass,
            launch_speed,
        })
    }
}
