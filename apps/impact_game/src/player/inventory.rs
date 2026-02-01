//! Player inventory.

use crate::{Game, define_lookup_type};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use roc_integration::roc;

#[derive(Clone, Debug)]
pub struct Inventory {
    mass: f32,
}

define_lookup_type! {
    variant = InventoryMass;
    #[roc(parents = "Lookup")]
    #[repr(C)]
    #[derive(Clone, Copy, Debug, Zeroable, Pod)]
    pub struct InventoryMass {
        mass: f32,
    }
}

impl InventoryMass {
    pub fn lookup(game: &Game) -> Result<Self> {
        Ok(Self {
            mass: game.player.inventory.mass(),
        })
    }
}

impl Inventory {
    pub fn empty() -> Self {
        Self { mass: 0.0 }
    }

    pub fn mass(&self) -> f32 {
        self.mass
    }

    pub fn add_mass(&mut self, additional_mass: f32) {
        self.mass += additional_mass;
    }
}
