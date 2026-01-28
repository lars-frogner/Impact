//! Player inventory.

#[derive(Clone, Debug)]
pub struct Inventory {
    mass: f32,
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
