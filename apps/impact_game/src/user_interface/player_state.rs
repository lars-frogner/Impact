//! Player info overlay.

use crate::{Game, player::Player};
use anyhow::Result;

#[derive(Debug)]
pub(crate) struct DisplayedPlayerState {
    pub acceleration: f32,
    pub inventory_mass: f32,
}

impl DisplayedPlayerState {
    pub(crate) fn gather(game: &Game) -> Result<Self> {
        let (force_vector, mass) = Player::with_rigid_body(game.engine(), |rigid_body| {
            Ok((*rigid_body.total_force(), rigid_body.mass()))
        })?;

        let acceleration_vector = force_vector / mass;
        let acceleration = acceleration_vector.norm();

        let inventory_mass = game.player.inventory.mass();

        Ok(Self {
            acceleration,
            inventory_mass,
        })
    }
}
