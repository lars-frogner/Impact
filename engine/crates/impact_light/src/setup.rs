//! Light setup.

use crate::{
    AmbientEmission, AmbientLight, AmbientLightID, LightFlags, LightManager,
    OmnidirectionalEmission, OmnidirectionalLight, OmnidirectionalLightID,
    ShadowableOmnidirectionalEmission, ShadowableOmnidirectionalLight,
    ShadowableOmnidirectionalLightID, ShadowableUnidirectionalEmission,
    ShadowableUnidirectionalLight, ShadowableUnidirectionalLightID, UnidirectionalEmission,
    UnidirectionalLight, UnidirectionalLightID,
};
use impact_math::angle::Degrees;
use nalgebra::{Isometry3, Point3, UnitQuaternion, UnitVector3};

pub fn setup_ambient_light(
    light_manager: &mut LightManager,
    ambient_emission: &AmbientEmission,
) -> AmbientLightID {
    let ambient_light = AmbientLight::new(crate::compute_luminance_for_uniform_illuminance(
        &ambient_emission.illuminance,
    ));

    let id = light_manager.add_ambient_light(ambient_light);

    id
}

pub fn setup_omnidirectional_light(
    light_manager: &mut LightManager,
    view_transform: &Isometry3<f32>,
    position: &Point3<f32>,
    omnidirectional_emission: &OmnidirectionalEmission,
    flags: LightFlags,
) -> OmnidirectionalLightID {
    let position = view_transform.transform_point(position);
    let omnidirectional_light = OmnidirectionalLight::new(
        position,
        omnidirectional_emission.luminous_intensity,
        f32::max(omnidirectional_emission.source_extent, 0.0),
        flags,
    );
    let id = light_manager.add_omnidirectional_light(omnidirectional_light);

    id
}

pub fn setup_shadowable_omnidirectional_light(
    light_manager: &mut LightManager,
    view_transform: &Isometry3<f32>,
    position: &Point3<f32>,
    omnidirectional_emission: &ShadowableOmnidirectionalEmission,
    flags: LightFlags,
) -> ShadowableOmnidirectionalLightID {
    let position = view_transform.transform_point(position);
    let omnidirectional_light = ShadowableOmnidirectionalLight::new(
        position,
        omnidirectional_emission.luminous_intensity,
        f32::max(omnidirectional_emission.source_extent, 0.0),
        flags,
    );
    let id = light_manager.add_shadowable_omnidirectional_light(omnidirectional_light);

    id
}

pub fn setup_unidirectional_light(
    light_manager: &mut LightManager,
    view_transform: &Isometry3<f32>,
    unidirectional_emission: &UnidirectionalEmission,
    flags: LightFlags,
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
    let id = light_manager.add_unidirectional_light(unidirectional_light);

    id
}

pub fn setup_shadowable_unidirectional_light(
    light_manager: &mut LightManager,
    view_transform: &Isometry3<f32>,
    unidirectional_emission: &ShadowableUnidirectionalEmission,
    flags: LightFlags,
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
    let id = light_manager.add_shadowable_unidirectional_light(unidirectional_light);

    id
}

pub fn sync_ambient_light_in_storage(
    light_manager: &mut LightManager,
    light_id: AmbientLightID,
    ambient_emission: &AmbientEmission,
) {
    light_manager.set_ambient_light_illuminance(light_id, ambient_emission.illuminance);
}

pub fn sync_omnidirectional_light_in_storage(
    light_manager: &mut LightManager,
    light_id: OmnidirectionalLightID,
    view_transform: &Isometry3<f32>,
    position: &Point3<f32>,
    omnidirectional_emission: &OmnidirectionalEmission,
    flags: LightFlags,
) {
    let light = light_manager.omnidirectional_light_mut(light_id);
    light.set_camera_space_position(view_transform.transform_point(position));
    light.set_luminous_intensity(omnidirectional_emission.luminous_intensity);
    light.set_emissive_extent(omnidirectional_emission.source_extent);
    light.set_flags(flags);
}

pub fn sync_shadowable_omnidirectional_light_in_storage(
    light_manager: &mut LightManager,
    light_id: ShadowableOmnidirectionalLightID,
    view_transform: &Isometry3<f32>,
    position: &Point3<f32>,
    omnidirectional_emission: &ShadowableOmnidirectionalEmission,
    flags: LightFlags,
) {
    let light = light_manager.shadowable_omnidirectional_light_mut(light_id);
    light.set_camera_space_position(view_transform.transform_point(position));
    light.set_luminous_intensity(omnidirectional_emission.luminous_intensity);
    light.set_emissive_extent(omnidirectional_emission.source_extent);
    light.set_flags(flags);
}

pub fn sync_unidirectional_light_in_storage(
    light_manager: &mut LightManager,
    light_id: UnidirectionalLightID,
    view_transform: &Isometry3<f32>,
    unidirectional_emission: &UnidirectionalEmission,
    flags: LightFlags,
) {
    let light = light_manager.unidirectional_light_mut(light_id);
    light.set_camera_space_direction(UnitVector3::new_unchecked(
        view_transform.transform_vector(&unidirectional_emission.direction),
    ));
    light.set_perpendicular_illuminance(unidirectional_emission.perpendicular_illuminance);
    light.set_angular_extent(unidirectional_emission.angular_source_extent);
    light.set_flags(flags);
}

pub fn sync_unidirectional_light_with_orientation_in_storage(
    light_manager: &mut LightManager,
    light_id: UnidirectionalLightID,
    view_transform: &Isometry3<f32>,
    orientation: &UnitQuaternion<f32>,
    unidirectional_emission: &UnidirectionalEmission,
    flags: LightFlags,
) {
    let world_direction = orientation.transform_vector(&unidirectional_emission.direction);

    let light = light_manager.unidirectional_light_mut(light_id);
    light.set_camera_space_direction(UnitVector3::new_unchecked(
        view_transform.transform_vector(&world_direction),
    ));
    light.set_perpendicular_illuminance(unidirectional_emission.perpendicular_illuminance);
    light.set_angular_extent(unidirectional_emission.angular_source_extent);
    light.set_flags(flags);
}

pub fn sync_shadowable_unidirectional_light_in_storage(
    light_manager: &mut LightManager,
    light_id: ShadowableUnidirectionalLightID,
    view_transform: &Isometry3<f32>,
    unidirectional_emission: &ShadowableUnidirectionalEmission,
    flags: LightFlags,
) {
    let light = light_manager.shadowable_unidirectional_light_mut(light_id);
    light.set_camera_space_direction(UnitVector3::new_unchecked(
        view_transform.transform_vector(&unidirectional_emission.direction),
    ));
    light.set_perpendicular_illuminance(unidirectional_emission.perpendicular_illuminance);
    light.set_angular_extent(unidirectional_emission.angular_source_extent);
    light.set_flags(flags);
}

pub fn sync_shadowable_unidirectional_light_with_orientation_in_storage(
    light_manager: &mut LightManager,
    light_id: ShadowableUnidirectionalLightID,
    view_transform: &Isometry3<f32>,
    orientation: &UnitQuaternion<f32>,
    unidirectional_emission: &ShadowableUnidirectionalEmission,
    flags: LightFlags,
) {
    let world_direction = orientation.transform_vector(&unidirectional_emission.direction);

    let light = light_manager.shadowable_unidirectional_light_mut(light_id);
    light.set_camera_space_direction(UnitVector3::new_unchecked(
        view_transform.transform_vector(&world_direction),
    ));
    light.set_perpendicular_illuminance(unidirectional_emission.perpendicular_illuminance);
    light.set_angular_extent(unidirectional_emission.angular_source_extent);
    light.set_flags(flags);
}
