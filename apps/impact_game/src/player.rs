//! Player character.

pub mod inventory;
pub mod tools;

use inventory::Inventory;

#[derive(Clone, Debug)]
pub struct Player {
    pub inventory: Inventory,
}

impl Player {
    pub fn new() -> Self {
        Self {
            inventory: Inventory::empty(),
        }
    }
}
