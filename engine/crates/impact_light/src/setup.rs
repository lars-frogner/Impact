//! Light setup.

use crate::{
    AmbientEmission, AmbientLight, AmbientLightID, LightFlags, LightStorage,
    OmnidirectionalEmission, OmnidirectionalLight, OmnidirectionalLightID,
    ShadowableOmnidirectionalEmission, ShadowableOmnidirectionalLight,
    ShadowableOmnidirectionalLightID, ShadowableUnidirectionalEmission,
    ShadowableUnidirectionalLight, ShadowableUnidirectionalLightID, UnidirectionalEmission,
    UnidirectionalLight, UnidirectionalLightID,
};
use impact_math::Degrees;
use nalgebra::{Point3, Similarity3, UnitQuaternion, UnitVector3};

pub fn setup_ambient_light(
    light_storage: &mut LightStorage,
    ambient_emission: &AmbientEmission,
    desynchronized: &mut bool,
) -> AmbientLightID {
    let ambient_light = AmbientLight::new(crate::compute_luminance_for_uniform_illuminance(
        &ambient_emission.illuminance,
    ));

    let id = light_storage.add_ambient_light(ambient_light);

    *desynchronized = true;

    id
}

pub fn setup_omnidirectional_light(
    light_storage: &mut LightStorage,
    view_transform: &Similarity3<f32>,
    position: &Point3<f32>,
    omnidirectional_emission: &OmnidirectionalEmission,
    flags: LightFlags,
    desynchronized: &mut bool,
) -> OmnidirectionalLightID {
    let position = view_transform.transform_point(position);
    let omnidirectional_light = OmnidirectionalLight::new(
        position,
        omnidirectional_emission.luminous_intensity,
        f32::max(omnidirectional_emission.source_extent, 0.0),
        flags,
    );
    let id = light_storage.add_omnidirectional_light(omnidirectional_light);

    *desynchronized = true;

    id
}

pub fn setup_shadowable_omnidirectional_light(
    light_storage: &mut LightStorage,
    view_transform: &Similarity3<f32>,
    position: &Point3<f32>,
    omnidirectional_emission: &ShadowableOmnidirectionalEmission,
    flags: LightFlags,
    desynchronized: &mut bool,
) -> ShadowableOmnidirectionalLightID {
    let position = view_transform.transform_point(position);
    let omnidirectional_light = ShadowableOmnidirectionalLight::new(
        position,
        omnidirectional_emission.luminous_intensity,
        f32::max(omnidirectional_emission.source_extent, 0.0),
        flags,
    );
    let id = light_storage.add_shadowable_omnidirectional_light(omnidirectional_light);

    *desynchronized = true;

    id
}

pub fn setup_unidirectional_light(
    light_storage: &mut LightStorage,
    view_transform: &Similarity3<f32>,
    unidirectional_emission: &UnidirectionalEmission,
    flags: LightFlags,
    desynchronized: &mut bool,
) -> UnidirectionalLightID {
    // The view transform contains no scaling, so the direction remains normalized
    let direction = UnitVector3::new_unchecked(
        view_transform.transform_vector(&unidirectional_emission.direction),
    );
    let unidirectional_light = UnidirectionalLight::new(
        direction,
        unidirectional_emission.perpendicular_illuminance,
        Degrees(f32::max(
            unidirectional_emission.angular_source_extent.0,
            0.0,
        )),
        flags,
    );
    let id = light_storage.add_unidirectional_light(unidirectional_light);

    *desynchronized = true;

    id
}

pub fn setup_shadowable_unidirectional_light(
    light_storage: &mut LightStorage,
    view_transform: &Similarity3<f32>,
    unidirectional_emission: &ShadowableUnidirectionalEmission,
    flags: LightFlags,
    desynchronized: &mut bool,
) -> ShadowableUnidirectionalLightID {
    // The view transform contains no scaling, so the direction remains normalized
    let direction = UnitVector3::new_unchecked(
        view_transform.transform_vector(&unidirectional_emission.direction),
    );
    let unidirectional_light = ShadowableUnidirectionalLight::new(
        direction,
        unidirectional_emission.perpendicular_illuminance,
        Degrees(f32::max(
            unidirectional_emission.angular_source_extent.0,
            0.0,
        )),
        flags,
    );
    let id = light_storage.add_shadowable_unidirectional_light(unidirectional_light);

    *desynchronized = true;

    id
}

pub fn sync_ambient_light_in_storage(
    light_storage: &mut LightStorage,
    light_id: AmbientLightID,
    ambient_emission: &AmbientEmission,
) {
    light_storage.set_ambient_light_illuminance(light_id, ambient_emission.illuminance);
}

pub fn sync_omnidirectional_light_in_storage(
    light_storage: &mut LightStorage,
    light_id: OmnidirectionalLightID,
    view_transform: &Similarity3<f32>,
    position: &Point3<f32>,
    omnidirectional_emission: &OmnidirectionalEmission,
    flags: LightFlags,
) {
    let light = light_storage.omnidirectional_light_mut(light_id);
    light.set_camera_space_position(view_transform.transform_point(position));
    light.set_luminous_intensity(omnidirectional_emission.luminous_intensity);
    light.set_emissive_extent(omnidirectional_emission.source_extent);
    light.set_flags(flags);
}

pub fn sync_shadowable_omnidirectional_light_in_storage(
    light_storage: &mut LightStorage,
    light_id: ShadowableOmnidirectionalLightID,
    view_transform: &Similarity3<f32>,
    position: &Point3<f32>,
    omnidirectional_emission: &ShadowableOmnidirectionalEmission,
    flags: LightFlags,
) {
    let light = light_storage.shadowable_omnidirectional_light_mut(light_id);
    light.set_camera_space_position(view_transform.transform_point(position));
    light.set_luminous_intensity(omnidirectional_emission.luminous_intensity);
    light.set_emissive_extent(omnidirectional_emission.source_extent);
    light.set_flags(flags);
}

pub fn sync_unidirectional_light_in_storage(
    light_storage: &mut LightStorage,
    light_id: UnidirectionalLightID,
    view_transform: &Similarity3<f32>,
    unidirectional_emission: &UnidirectionalEmission,
    flags: LightFlags,
) {
    let light = light_storage.unidirectional_light_mut(light_id);
    light.set_camera_space_direction(UnitVector3::new_unchecked(
        view_transform.transform_vector(&unidirectional_emission.direction),
    ));
    light.set_perpendicular_illuminance(unidirectional_emission.perpendicular_illuminance);
    light.set_angular_extent(unidirectional_emission.angular_source_extent);
    light.set_flags(flags);
}

pub fn sync_unidirectional_light_with_orientation_in_storage(
    light_storage: &mut LightStorage,
    light_id: UnidirectionalLightID,
    view_transform: &Similarity3<f32>,
    orientation: &UnitQuaternion<f32>,
    unidirectional_emission: &UnidirectionalEmission,
    flags: LightFlags,
) {
    let world_direction = orientation.transform_vector(&unidirectional_emission.direction);

    let light = light_storage.unidirectional_light_mut(light_id);
    light.set_camera_space_direction(UnitVector3::new_unchecked(
        view_transform.transform_vector(&world_direction),
    ));
    light.set_perpendicular_illuminance(unidirectional_emission.perpendicular_illuminance);
    light.set_angular_extent(unidirectional_emission.angular_source_extent);
    light.set_flags(flags);
}

pub fn sync_shadowable_unidirectional_light_in_storage(
    light_storage: &mut LightStorage,
    light_id: ShadowableUnidirectionalLightID,
    view_transform: &Similarity3<f32>,
    unidirectional_emission: &ShadowableUnidirectionalEmission,
    flags: LightFlags,
) {
    let light = light_storage.shadowable_unidirectional_light_mut(light_id);
    light.set_camera_space_direction(UnitVector3::new_unchecked(
        view_transform.transform_vector(&unidirectional_emission.direction),
    ));
    light.set_perpendicular_illuminance(unidirectional_emission.perpendicular_illuminance);
    light.set_angular_extent(unidirectional_emission.angular_source_extent);
    light.set_flags(flags);
}

pub fn sync_shadowable_unidirectional_light_with_orientation_in_storage(
    light_storage: &mut LightStorage,
    light_id: ShadowableUnidirectionalLightID,
    view_transform: &Similarity3<f32>,
    orientation: &UnitQuaternion<f32>,
    unidirectional_emission: &ShadowableUnidirectionalEmission,
    flags: LightFlags,
) {
    let world_direction = orientation.transform_vector(&unidirectional_emission.direction);

    let light = light_storage.shadowable_unidirectional_light_mut(light_id);
    light.set_camera_space_direction(UnitVector3::new_unchecked(
        view_transform.transform_vector(&world_direction),
    ));
    light.set_perpendicular_illuminance(unidirectional_emission.perpendicular_illuminance);
    light.set_angular_extent(unidirectional_emission.angular_source_extent);
    light.set_flags(flags);
}

/// Checks if the given entity has a light component, and if so, removes the
/// assocated light from the given [`LightStorage`].
#[cfg(feature = "ecs")]
pub fn cleanup_light_for_removed_entity(
    light_storage: &std::sync::RwLock<LightStorage>,
    entity: &impact_ecs::world::EntityEntry<'_>,
    desynchronized: &mut bool,
) {
    cleanup_ambient_light_for_removed_entity(light_storage, entity, desynchronized);
    cleanup_omnidirectional_light_for_removed_entity(light_storage, entity, desynchronized);
    cleanup_unidirectional_light_for_removed_entity(light_storage, entity, desynchronized);
}

/// Checks if the given entity has a [`AmbientLightHandle`], and if so, removes
/// the assocated [`AmbientLight`] from the given [`LightStorage`].
#[cfg(feature = "ecs")]
fn cleanup_ambient_light_for_removed_entity(
    light_storage: &std::sync::RwLock<LightStorage>,
    entity: &impact_ecs::world::EntityEntry<'_>,
    desynchronized: &mut bool,
) {
    if let Some(light_id) = entity.get_component::<AmbientLightID>() {
        let light_id = *light_id.access();
        light_storage
            .write()
            .unwrap()
            .remove_ambient_light(light_id);
        *desynchronized = true;
    }
}

/// Checks if the given entity has a [`OmnidirectionalLightHandle`] or
/// [`ShadowableOmnidirectionalLightHandle`], and if so, removes the assocated
/// [`OmnidirectionalLight`] or [`ShadowableOmnidirectionalLight`] from the
/// given [`LightStorage`].
#[cfg(feature = "ecs")]
fn cleanup_omnidirectional_light_for_removed_entity(
    light_storage: &std::sync::RwLock<LightStorage>,
    entity: &impact_ecs::world::EntityEntry<'_>,
    desynchronized: &mut bool,
) {
    if let Some(light_id) = entity.get_component::<OmnidirectionalLightID>() {
        let light_id = *light_id.access();
        light_storage
            .write()
            .unwrap()
            .remove_omnidirectional_light(light_id);
        *desynchronized = true;
    }

    if let Some(light_id) = entity.get_component::<ShadowableOmnidirectionalLightID>() {
        let light_id = *light_id.access();
        light_storage
            .write()
            .unwrap()
            .remove_shadowable_omnidirectional_light(light_id);
        *desynchronized = true;
    }
}

/// Checks if the given entity has a [`UnidirectionalLightHandle`] or
/// [`ShadowableUnidirectionalLightHandle`], and if so, removes the assocated
/// [`UnidirectionalLight`] or [`ShadowableUnidirectionalLight`] from the given
/// [`LightStorage`].
#[cfg(feature = "ecs")]
fn cleanup_unidirectional_light_for_removed_entity(
    light_storage: &std::sync::RwLock<LightStorage>,
    entity: &impact_ecs::world::EntityEntry<'_>,
    desynchronized: &mut bool,
) {
    if let Some(light_id) = entity.get_component::<UnidirectionalLightID>() {
        let light_id = *light_id.access();
        light_storage
            .write()
            .unwrap()
            .remove_unidirectional_light(light_id);
        *desynchronized = true;
    }

    if let Some(light_id) = entity.get_component::<ShadowableUnidirectionalLightID>() {
        let light_id = *light_id.access();
        light_storage
            .write()
            .unwrap()
            .remove_shadowable_unidirectional_light(light_id);
        *desynchronized = true;
    }
}
