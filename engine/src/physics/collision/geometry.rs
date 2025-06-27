//! Collidable geometry implementations.

pub mod basic;
pub mod plane;
pub mod sphere;
pub mod voxel;

use crate::physics::{collision::CollidableID, constraint::contact::ContactID};

pub fn contact_id_from_collidable_ids(a: CollidableID, b: CollidableID) -> ContactID {
    ContactID::from_two_u32(a.0, b.0)
}
