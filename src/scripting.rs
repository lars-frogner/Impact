//! Interaction with scripts.

#[derive(Debug)]
pub struct Callbacks {
    pub setup_scene: fn(),
}

impl Default for Callbacks {
    fn default() -> Self {
        Self { setup_scene: || {} }
    }
}
