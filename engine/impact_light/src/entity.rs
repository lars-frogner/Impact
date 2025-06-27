//! Management of lights for entities.

use crate::{
    AmbientLight, LightFlags, LightStorage, OmnidirectionalLight, ShadowableOmnidirectionalLight,
    ShadowableUnidirectionalLight, UnidirectionalLight,
    components::{
        AmbientEmissionComp, AmbientLightComp, OmnidirectionalEmissionComp,
        OmnidirectionalLightComp, ShadowableOmnidirectionalEmissionComp,
        ShadowableOmnidirectionalLightComp, ShadowableUnidirectionalEmissionComp,
        ShadowableUnidirectionalLightComp, UnidirectionalEmissionComp, UnidirectionalLightComp,
    },
};
use impact_ecs::world::EntityEntry;
use impact_math::Degrees;
use nalgebra::{Point3, Similarity3, UnitQuaternion, UnitVector3};
use std::sync::RwLock;

pub fn setup_ambient_light(
    light_storage: &mut LightStorage,
    ambient_emission: &AmbientEmissionComp,
    desynchronized: &mut bool,
) -> AmbientLightComp {
    let ambient_light = AmbientLight::new(crate::compute_luminance_for_uniform_illuminance(
        &ambient_emission.illuminance,
    ));

    let id = light_storage.add_ambient_light(ambient_light);

    *desynchronized = true;

    AmbientLightComp { id }
}

pub fn setup_omnidirectional_light(
    light_storage: &mut LightStorage,
    view_transform: &Similarity3<f32>,
    position: &Point3<f32>,
    omnidirectional_emission: &OmnidirectionalEmissionComp,
    flags: LightFlags,
    desynchronized: &mut bool,
) -> OmnidirectionalLightComp {
    let position = view_transform.transform_point(position);
    let omnidirectional_light = OmnidirectionalLight::new(
        position,
        omnidirectional_emission.luminous_intensity,
        f32::max(omnidirectional_emission.source_extent, 0.0),
        flags,
    );
    let id = light_storage.add_omnidirectional_light(omnidirectional_light);

    *desynchronized = true;

    OmnidirectionalLightComp { id }
}

pub fn setup_shadowable_omnidirectional_light(
    light_storage: &mut LightStorage,
    view_transform: &Similarity3<f32>,
    position: &Point3<f32>,
    omnidirectional_emission: &ShadowableOmnidirectionalEmissionComp,
    flags: LightFlags,
    desynchronized: &mut bool,
) -> ShadowableOmnidirectionalLightComp {
    let position = view_transform.transform_point(position);
    let omnidirectional_light = ShadowableOmnidirectionalLight::new(
        position,
        omnidirectional_emission.luminous_intensity,
        f32::max(omnidirectional_emission.source_extent, 0.0),
        flags,
    );
    let id = light_storage.add_shadowable_omnidirectional_light(omnidirectional_light);

    *desynchronized = true;

    ShadowableOmnidirectionalLightComp { id }
}

pub fn setup_unidirectional_light(
    light_storage: &mut LightStorage,
    view_transform: &Similarity3<f32>,
    unidirectional_emission: &UnidirectionalEmissionComp,
    flags: LightFlags,
    desynchronized: &mut bool,
) -> UnidirectionalLightComp {
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

    UnidirectionalLightComp { id }
}

pub fn setup_shadowable_unidirectional_light(
    light_storage: &mut LightStorage,
    view_transform: &Similarity3<f32>,
    unidirectional_emission: &ShadowableUnidirectionalEmissionComp,
    flags: LightFlags,
    desynchronized: &mut bool,
) -> ShadowableUnidirectionalLightComp {
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

    ShadowableUnidirectionalLightComp { id }
}

pub fn sync_ambient_light_in_storage(
    light_storage: &mut LightStorage,
    ambient_light: &AmbientLightComp,
    ambient_emission: &AmbientEmissionComp,
) {
    light_storage.set_ambient_light_illuminance(ambient_light.id, ambient_emission.illuminance);
}

pub fn sync_omnidirectional_light_in_storage(
    light_storage: &mut LightStorage,
    omnidirectional_light: &OmnidirectionalLightComp,
    view_transform: &Similarity3<f32>,
    position: &Point3<f32>,
    omnidirectional_emission: &OmnidirectionalEmissionComp,
    flags: LightFlags,
) {
    let light_id = omnidirectional_light.id;
    let light = light_storage.omnidirectional_light_mut(light_id);
    light.set_camera_space_position(view_transform.transform_point(position));
    light.set_luminous_intensity(omnidirectional_emission.luminous_intensity);
    light.set_emissive_extent(omnidirectional_emission.source_extent);
    light.set_flags(flags);
}

pub fn sync_shadowable_omnidirectional_light_in_storage(
    light_storage: &mut LightStorage,
    omnidirectional_light: &ShadowableOmnidirectionalLightComp,
    view_transform: &Similarity3<f32>,
    position: &Point3<f32>,
    omnidirectional_emission: &ShadowableOmnidirectionalEmissionComp,
    flags: LightFlags,
) {
    let light_id = omnidirectional_light.id;
    let light = light_storage.shadowable_omnidirectional_light_mut(light_id);
    light.set_camera_space_position(view_transform.transform_point(position));
    light.set_luminous_intensity(omnidirectional_emission.luminous_intensity);
    light.set_emissive_extent(omnidirectional_emission.source_extent);
    light.set_flags(flags);
}

pub fn sync_unidirectional_light_in_storage(
    light_storage: &mut LightStorage,
    unidirectional_light: &UnidirectionalLightComp,
    view_transform: &Similarity3<f32>,
    unidirectional_emission: &UnidirectionalEmissionComp,
    flags: LightFlags,
) {
    let light_id = unidirectional_light.id;
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
    unidirectional_light: &UnidirectionalLightComp,
    view_transform: &Similarity3<f32>,
    orientation: &UnitQuaternion<f32>,
    unidirectional_emission: &UnidirectionalEmissionComp,
    flags: LightFlags,
) {
    let world_direction = orientation.transform_vector(&unidirectional_emission.direction);

    let light_id = unidirectional_light.id;
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
    unidirectional_light: &ShadowableUnidirectionalLightComp,
    view_transform: &Similarity3<f32>,
    unidirectional_emission: &ShadowableUnidirectionalEmissionComp,
    flags: LightFlags,
) {
    let light_id = unidirectional_light.id;
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
    unidirectional_light: &ShadowableUnidirectionalLightComp,
    view_transform: &Similarity3<f32>,
    orientation: &UnitQuaternion<f32>,
    unidirectional_emission: &ShadowableUnidirectionalEmissionComp,
    flags: LightFlags,
) {
    let world_direction = orientation.transform_vector(&unidirectional_emission.direction);

    let light_id = unidirectional_light.id;
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
pub fn cleanup_light_for_removed_entity(
    light_storage: &RwLock<LightStorage>,
    entity: &EntityEntry<'_>,
    desynchronized: &mut bool,
) {
    cleanup_ambient_light_for_removed_entity(light_storage, entity, desynchronized);
    cleanup_omnidirectional_light_for_removed_entity(light_storage, entity, desynchronized);
    cleanup_unidirectional_light_for_removed_entity(light_storage, entity, desynchronized);
}

/// Checks if the given entity has a [`AmbientLightComp`], and if so, removes
/// the assocated [`AmbientLight`] from the given [`LightStorage`].
fn cleanup_ambient_light_for_removed_entity(
    light_storage: &RwLock<LightStorage>,
    entity: &EntityEntry<'_>,
    desynchronized: &mut bool,
) {
    if let Some(ambient_light) = entity.get_component::<AmbientLightComp>() {
        let light_id = ambient_light.access().id;
        light_storage
            .write()
            .unwrap()
            .remove_ambient_light(light_id);
        *desynchronized = true;
    }
}

/// Checks if the given entity has a [`OmnidirectionalLightComp`] or
/// [`ShadowableOmnidirectionalLightComp`], and if so, removes the assocated
/// [`OmnidirectionalLight`] or [`ShadowableOmnidirectionalLight`] from the
/// given [`LightStorage`].
fn cleanup_omnidirectional_light_for_removed_entity(
    light_storage: &RwLock<LightStorage>,
    entity: &EntityEntry<'_>,
    desynchronized: &mut bool,
) {
    if let Some(omnidirectional_light) = entity.get_component::<OmnidirectionalLightComp>() {
        let light_id = omnidirectional_light.access().id;
        light_storage
            .write()
            .unwrap()
            .remove_omnidirectional_light(light_id);
        *desynchronized = true;
    }

    if let Some(omnidirectional_light) =
        entity.get_component::<ShadowableOmnidirectionalLightComp>()
    {
        let light_id = omnidirectional_light.access().id;
        light_storage
            .write()
            .unwrap()
            .remove_shadowable_omnidirectional_light(light_id);
        *desynchronized = true;
    }
}

/// Checks if the given entity has a [`UnidirectionalLightComp`] or
/// [`ShadowableUnidirectionalLightComp`], and if so, removes the assocated
/// [`UnidirectionalLight`] or [`ShadowableUnidirectionalLight`] from the given
/// [`LightStorage`].
fn cleanup_unidirectional_light_for_removed_entity(
    light_storage: &RwLock<LightStorage>,
    entity: &EntityEntry<'_>,
    desynchronized: &mut bool,
) {
    if let Some(unidirectional_light) = entity.get_component::<UnidirectionalLightComp>() {
        let light_id = unidirectional_light.access().id;
        light_storage
            .write()
            .unwrap()
            .remove_unidirectional_light(light_id);
        *desynchronized = true;
    }

    if let Some(unidirectional_light) = entity.get_component::<ShadowableUnidirectionalLightComp>()
    {
        let light_id = unidirectional_light.access().id;
        light_storage
            .write()
            .unwrap()
            .remove_shadowable_unidirectional_light(light_id);
        *desynchronized = true;
    }
}
