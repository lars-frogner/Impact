//! Collidable implementations.

pub mod basic;
pub mod plane;
pub mod sphere;

use crate::{collision::CollidableID, constraint::contact::ContactID};

pub fn contact_id_from_collidable_ids(a: CollidableID, b: CollidableID) -> ContactID {
    ContactID::from_two_u32(a.0, b.0)
}

pub fn contact_id_from_collidable_ids_and_indices(
    a: CollidableID,
    b: CollidableID,
    indices: [usize; 3],
) -> ContactID {
    ContactID::from_two_u32_and_three_indices(a.0, b.0, indices)
}
