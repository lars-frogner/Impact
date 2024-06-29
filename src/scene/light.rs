//! Management of lights.

mod ambient_light;
mod components;
mod omnidirectional_light;
mod unidirectional_light;

pub use ambient_light::AmbientLight;
pub use components::{
    register_light_components, AmbientEmissionComp, AmbientLightComp, OmnidirectionalEmissionComp,
    OmnidirectionalLightComp, UnidirectionalEmissionComp, UnidirectionalLightComp,
};
pub use omnidirectional_light::OmnidirectionalLight;
pub use unidirectional_light::{UnidirectionalLight, MAX_SHADOW_MAP_CASCADES};

use crate::{
    geometry::{InstanceFeatureBufferRangeID, UniformBuffer},
    num::Float,
    rendering::fre,
};
use bytemuck::{Pod, Zeroable};
use nalgebra::Vector3;

/// The luminous intensity of a light source, which is the visible power
/// (luminous flux) emitted per unit solid angle, represented as an RGB triplet.
pub type LuminousIntensity = Vector3<fre>;

/// The illuminance of surface, which is the visible power (luminous flux)
/// received per unit area, represented as an RGB triplet.
pub type Illumninance = Vector3<fre>;

/// A luminance, which is the visible power (luminous flux) per unit solid angle
/// and area of light traveling in a given direction, represented as an RGB
/// triplet.
pub type Luminance = Vector3<fre>;

/// Identifier for a light in a [`LightStorage`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct LightID(u32);

/// A type of light source.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum LightType {
    AmbientLight,
    OmnidirectionalLight,
    UnidirectionalLight,
}

type LightUniformBuffer<L> = UniformBuffer<LightID, L>;
type AmbientLightUniformBuffer = LightUniformBuffer<AmbientLight>;
type OmnidirectionalLightUniformBuffer = LightUniformBuffer<OmnidirectionalLight>;
type UnidirectionalLightUniformBuffer = LightUniformBuffer<UnidirectionalLight>;

/// Container for all light sources in a scene.
#[derive(Debug)]
pub struct LightStorage {
    ambient_light_buffer: AmbientLightUniformBuffer,
    omnidirectional_light_buffer: OmnidirectionalLightUniformBuffer,
    unidirectional_light_buffer: UnidirectionalLightUniformBuffer,
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
            ambient_light_buffer: AmbientLightUniformBuffer::with_capacity(1),
            omnidirectional_light_buffer: OmnidirectionalLightUniformBuffer::with_capacity(
                Self::INITIAL_LIGHT_CAPACITY,
            ),
            unidirectional_light_buffer: UnidirectionalLightUniformBuffer::with_capacity(
                Self::INITIAL_LIGHT_CAPACITY,
            ),
            light_id_counter: 0,
        }
    }

    /// Returns a reference to the [`UniformBuffer`] holding all
    /// [`AmbientLight`]s.
    pub fn ambient_light_buffer(&self) -> &UniformBuffer<LightID, AmbientLight> {
        &self.ambient_light_buffer
    }

    /// Returns a reference to the [`UniformBuffer`] holding all
    /// [`OmnidirectionalLight`]s.
    pub fn omnidirectional_light_buffer(&self) -> &UniformBuffer<LightID, OmnidirectionalLight> {
        &self.omnidirectional_light_buffer
    }

    /// Returns a reference to the [`UniformBuffer`] holding all
    /// [`UnidirectionalLight`]s.
    pub fn unidirectional_light_buffer(&self) -> &UniformBuffer<LightID, UnidirectionalLight> {
        &self.unidirectional_light_buffer
    }

    /// Adds the given [`AmbientLight`] to the storage.
    ///
    /// # Returns
    /// A new [`LightID`] representing the added light source.
    pub fn add_ambient_light(&mut self, ambient_light: AmbientLight) -> LightID {
        let light_id = self.create_new_light_id();
        self.ambient_light_buffer
            .add_uniform(light_id, ambient_light);
        light_id
    }

    /// Adds the given [`OmnidirectionalLight`] to the storage.
    ///
    /// # Returns
    /// A new [`LightID`] representing the added light source.
    pub fn add_omnidirectional_light(
        &mut self,
        omnidirectional_light: OmnidirectionalLight,
    ) -> LightID {
        let light_id = self.create_new_light_id();
        self.omnidirectional_light_buffer
            .add_uniform(light_id, omnidirectional_light);
        light_id
    }

    /// Adds the given [`UnidirectionalLight`] to the storage.
    ///
    /// # Returns
    /// A new [`LightID`] representing the added light source.
    pub fn add_unidirectional_light(
        &mut self,
        unidirectional_light: UnidirectionalLight,
    ) -> LightID {
        let light_id = self.create_new_light_id();
        self.unidirectional_light_buffer
            .add_uniform(light_id, unidirectional_light);
        light_id
    }

    /// Removes the [`AmbientLight`] with the given ID from the storage.
    ///
    /// # Panics
    /// If no ambient light with the given ID exists.
    pub fn remove_ambient_light(&mut self, light_id: LightID) {
        self.ambient_light_buffer.remove_uniform(light_id);
    }

    /// Removes the [`OmnidirectionalLight`] with the given ID from the storage.
    ///
    /// # Panics
    /// If no omnidirectional light with the given ID exists.
    pub fn remove_omnidirectional_light(&mut self, light_id: LightID) {
        self.omnidirectional_light_buffer.remove_uniform(light_id);
    }

    /// Removes the [`UnidirectionalLight`] with the given ID from the storage.
    ///
    /// # Panics
    /// If no unidirectional light with the given ID exists.
    pub fn remove_unidirectional_light(&mut self, light_id: LightID) {
        self.unidirectional_light_buffer.remove_uniform(light_id);
    }

    /// Returns a mutable reference to the [`AmbientLight`] with the given ID.
    ///
    /// # Panics
    /// If no ambient light with the given ID exists.
    pub fn ambient_light_mut(&mut self, light_id: LightID) -> &mut AmbientLight {
        self.ambient_light_buffer
            .get_uniform_mut(light_id)
            .expect("Requested missing ambient light")
    }

    /// Returns a mutable reference to the [`OmnidirectionalLight`] with the given ID.
    ///
    /// # Panics
    /// If no omnidirectional light with the given ID exists.
    pub fn omnidirectional_light_mut(&mut self, light_id: LightID) -> &mut OmnidirectionalLight {
        self.omnidirectional_light_buffer
            .get_uniform_mut(light_id)
            .expect("Requested missing omnidirectional light")
    }

    /// Returns a mutable reference to the [`UnidirectionalLight`] with the given
    /// ID.
    ///
    /// # Panics
    /// If no unidirectional light with the given ID exists.
    pub fn unidirectional_light_mut(&mut self, light_id: LightID) -> &mut UnidirectionalLight {
        self.unidirectional_light_buffer
            .get_uniform_mut(light_id)
            .expect("Requested missing unidirectional light")
    }

    /// Returns an iterator over the ambient lights in the storage where each
    /// item contains the light ID and a mutable reference to the light.
    pub fn ambient_lights_with_ids_mut(
        &mut self,
    ) -> impl Iterator<Item = (LightID, &mut AmbientLight)> {
        self.ambient_light_buffer.valid_uniforms_with_ids_mut()
    }

    /// Returns an iterator over the omnidirectional lights in the storage where
    /// each item contains the light ID and a mutable reference to the light.
    pub fn omnidirectional_lights_with_ids_mut(
        &mut self,
    ) -> impl Iterator<Item = (LightID, &mut OmnidirectionalLight)> {
        self.omnidirectional_light_buffer
            .valid_uniforms_with_ids_mut()
    }

    /// Returns an iterator over the unidirectional lights in the storage where
    /// each item contains the light ID and a mutable reference to the light.
    pub fn unidirectional_lights_with_ids_mut(
        &mut self,
    ) -> impl Iterator<Item = (LightID, &mut UnidirectionalLight)> {
        self.unidirectional_light_buffer
            .valid_uniforms_with_ids_mut()
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

/// Computes the isotropic luminance incident on any surface in a light field
/// with the given uniform illuminance.
pub fn compute_luminance_for_uniform_illuminance(illuminance: &Illumninance) -> Luminance {
    illuminance * fre::FRAC_1_PI
}
