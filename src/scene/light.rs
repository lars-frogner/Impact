//! Management of lights.

mod components;
mod point_light;

pub use components::{Omnidirectional, PointLightComp, RadianceComp};
pub use point_light::PointLight;

use crate::{geometry::UniformBuffer, rendering::fre};
use bytemuck::{Pod, Zeroable};
use nalgebra::Vector3;

/// The RGB radiance of a light source.
pub type Radiance = Vector3<fre>;

/// Identifier for a light in a [`LightStorage`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct LightID(u32);

type LightUniformBuffer<L> = UniformBuffer<LightID, L>;
type PointLightUniformBuffer = LightUniformBuffer<PointLight>;

/// Container for all light sources in a scene.
#[derive(Debug)]
pub struct LightStorage {
    point_light_buffer: PointLightUniformBuffer,
    light_id_counter: u32,
}

impl LightStorage {
    /// By creating light uniform buffers with a small initial capacity, we
    /// avoid excessive buffer reallocation when the first few lights are added.
    const INITIAL_LIGHT_CAPACITY: usize = 5;

    /// Creates a new empty light storage.
    pub fn new() -> Self {
        Self {
            point_light_buffer: PointLightUniformBuffer::with_capacity(
                Self::INITIAL_LIGHT_CAPACITY,
            ),
            light_id_counter: 0,
        }
    }

    /// Returns a reference to the [`UniformBuffer`] holding all
    /// [`PointLight`]s.
    pub fn point_light_buffer(&self) -> &UniformBuffer<LightID, PointLight> {
        &self.point_light_buffer
    }

    /// Adds the given [`PointLight`] to the storage.
    ///
    /// # Returns
    /// A new [`LightID`] representing the added light source.
    pub fn add_point_light(&mut self, point_light: PointLight) -> LightID {
        let light_id = self.create_new_light_id();
        self.point_light_buffer.add_uniform(light_id, point_light);
        light_id
    }

    /// Removes the [`PointLight`] with the given ID from the storage.
    ///
    /// # Panics
    /// If no point light with the given ID exists.
    pub fn remove_point_light(&mut self, light_id: LightID) {
        self.point_light_buffer.remove_uniform(light_id);
    }

    /// Returns a mutable reference to the [`PointLight`] with the given ID.
    ///
    /// # Panics
    /// If no point light with the given ID exists.
    pub fn point_light_mut(&mut self, light_id: LightID) -> &mut PointLight {
        self.point_light_buffer
            .get_uniform_mut(light_id)
            .expect("Requested missing point light")
    }

    fn create_new_light_id(&mut self) -> LightID {
        let light_id = LightID(self.light_id_counter);
        self.light_id_counter = self.light_id_counter.checked_add(1).unwrap();
        light_id
    }
}

impl Default for LightStorage {
    fn default() -> Self {
        Self::new()
    }
}
