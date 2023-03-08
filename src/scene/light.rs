//! Management of lights.

mod components;
mod directional_light;
mod point_light;

pub use components::{
    DirectionComp, DirectionalLightComp, Omnidirectional, PointLightComp, RadianceComp,
};
pub use directional_light::DirectionalLight;
pub use point_light::PointLight;

use crate::{
    geometry::{InstanceFeatureBufferRangeID, UniformBuffer},
    rendering::fre,
};
use bytemuck::{Pod, Zeroable};
use nalgebra::{UnitVector3, Vector3};

/// The direction of a directional light source.
pub type LightDirection = UnitVector3<fre>;

/// The RGB radiance of a light source.
pub type Radiance = Vector3<fre>;

/// Identifier for a light in a [`LightStorage`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct LightID(u32);

/// A type of light source.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum LightType {
    PointLight,
    DirectionalLight,
}

type LightUniformBuffer<L> = UniformBuffer<LightID, L>;
type PointLightUniformBuffer = LightUniformBuffer<PointLight>;
type DirectionalLightUniformBuffer = LightUniformBuffer<DirectionalLight>;

/// Container for all light sources in a scene.
#[derive(Debug)]
pub struct LightStorage {
    point_light_buffer: PointLightUniformBuffer,
    directional_light_buffer: DirectionalLightUniformBuffer,
    light_id_counter: u32,
}

impl LightID {
    /// Converts the light ID into an [`InstanceFeatureBufferRangeID`].
    pub fn as_instance_feature_buffer_range_id(&self) -> InstanceFeatureBufferRangeID {
        // Use a stride of 6 so that the ID can be incremented up to 5 times to
        // create additional ranges associated with the same light
        6 * self.0
    }
}

impl std::fmt::Display for LightID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
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
            directional_light_buffer: DirectionalLightUniformBuffer::with_capacity(
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

    /// Returns a reference to the [`UniformBuffer`] holding all
    /// [`DirectionalLight`]s.
    pub fn directional_light_buffer(&self) -> &UniformBuffer<LightID, DirectionalLight> {
        &self.directional_light_buffer
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

    /// Adds the given [`DirectionalLight`] to the storage.
    ///
    /// # Returns
    /// A new [`LightID`] representing the added light source.
    pub fn add_directional_light(&mut self, directional_light: DirectionalLight) -> LightID {
        let light_id = self.create_new_light_id();
        self.directional_light_buffer
            .add_uniform(light_id, directional_light);
        light_id
    }

    /// Removes the [`PointLight`] with the given ID from the storage.
    ///
    /// # Panics
    /// If no point light with the given ID exists.
    pub fn remove_point_light(&mut self, light_id: LightID) {
        self.point_light_buffer.remove_uniform(light_id);
    }

    /// Removes the [`DirectionalLight`] with the given ID from the storage.
    ///
    /// # Panics
    /// If no directional light with the given ID exists.
    pub fn remove_directional_light(&mut self, light_id: LightID) {
        self.directional_light_buffer.remove_uniform(light_id);
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

    /// Returns a mutable reference to the [`DirectionalLight`] with the given
    /// ID.
    ///
    /// # Panics
    /// If no directional light with the given ID exists.
    pub fn directional_light_mut(&mut self, light_id: LightID) -> &mut DirectionalLight {
        self.directional_light_buffer
            .get_uniform_mut(light_id)
            .expect("Requested missing directional light")
    }

    /// Returns an iterator over the point lights in the storage where each item
    /// contains the light ID and a mutable reference to the light.
    pub fn point_lights_with_ids_mut(
        &mut self,
    ) -> impl Iterator<Item = (LightID, &mut PointLight)> {
        self.point_light_buffer.valid_uniforms_with_ids_mut()
    }

    /// Returns an iterator over the directional lights in the storage where
    /// each item contains the light ID and a mutable reference to the light.
    pub fn directional_lights_with_ids_mut(
        &mut self,
    ) -> impl Iterator<Item = (LightID, &mut DirectionalLight)> {
        self.directional_light_buffer.valid_uniforms_with_ids_mut()
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
