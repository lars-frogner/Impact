//! Light setup.

use crate::{
    AmbientEmission, AmbientLight, AmbientLightID, LightFlags, LightManager,
    OmnidirectionalEmission, OmnidirectionalLight, OmnidirectionalLightID,
    ShadowableOmnidirectionalEmission, ShadowableOmnidirectionalLight,
    ShadowableOmnidirectionalLightID, ShadowableUnidirectionalEmission,
    ShadowableUnidirectionalLight, ShadowableUnidirectionalLightID, UnidirectionalEmission,
    UnidirectionalLight, UnidirectionalLightID,
};
use impact_math::{
    angle::Degrees, point::Point3, quaternion::UnitQuaternion, transform::Isometry3,
};

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
    view_transform: &Isometry3,
    position: &Point3,
    omnidirectional_emission: &OmnidirectionalEmission,
    flags: LightFlags,
) -> OmnidirectionalLightID {
    let position = view_transform.transform_point(position);
    let omnidirectional_light = OmnidirectionalLight::new(
        position.pack(),
        omnidirectional_emission.luminous_intensity,
        f32::max(omnidirectional_emission.source_extent, 0.0),
        flags,
    );
    let id = light_manager.add_omnidirectional_light(omnidirectional_light);

    id
}

pub fn setup_shadowable_omnidirectional_light(
    light_manager: &mut LightManager,
    view_transform: &Isometry3,
    position: &Point3,
    omnidirectional_emission: &ShadowableOmnidirectionalEmission,
    flags: LightFlags,
) -> ShadowableOmnidirectionalLightID {
    let position = view_transform.transform_point(position);
    let omnidirectional_light = ShadowableOmnidirectionalLight::new(
        position.pack(),
        omnidirectional_emission.luminous_intensity,
        f32::max(omnidirectional_emission.source_extent, 0.0),
        flags,
    );
    let id = light_manager.add_shadowable_omnidirectional_light(omnidirectional_light);

    id
}

pub fn setup_unidirectional_light(
    light_manager: &mut LightManager,
    view_transform: &Isometry3,
    unidirectional_emission: &UnidirectionalEmission,
    flags: LightFlags,
) -> UnidirectionalLightID {
    let direction = unidirectional_emission.direction.unpack();
    let camera_space_direction = view_transform.transform_unit_vector(&direction);

    let unidirectional_light = UnidirectionalLight::new(
        camera_space_direction.pack(),
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
    view_transform: &Isometry3,
    unidirectional_emission: &ShadowableUnidirectionalEmission,
    flags: LightFlags,
) -> ShadowableUnidirectionalLightID {
    let direction = unidirectional_emission.direction.unpack();
    let camera_space_direction = view_transform.transform_unit_vector(&direction);

    let unidirectional_light = ShadowableUnidirectionalLight::new(
        camera_space_direction.pack(),
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
    view_transform: &Isometry3,
    position: &Point3,
    omnidirectional_emission: &OmnidirectionalEmission,
    flags: LightFlags,
) {
    let light = light_manager.omnidirectional_light_mut(light_id);
    light.set_camera_space_position(view_transform.transform_point(position).pack());
    light.set_luminous_intensity(omnidirectional_emission.luminous_intensity);
    light.set_emissive_extent(omnidirectional_emission.source_extent);
    light.set_flags(flags);
}

pub fn sync_shadowable_omnidirectional_light_in_storage(
    light_manager: &mut LightManager,
    light_id: ShadowableOmnidirectionalLightID,
    view_transform: &Isometry3,
    position: &Point3,
    omnidirectional_emission: &ShadowableOmnidirectionalEmission,
    flags: LightFlags,
) {
    let light = light_manager.shadowable_omnidirectional_light_mut(light_id);
    light.set_camera_space_position(view_transform.transform_point(position).pack());
    light.set_luminous_intensity(omnidirectional_emission.luminous_intensity);
    light.set_emissive_extent(omnidirectional_emission.source_extent);
    light.set_flags(flags);
}

pub fn sync_unidirectional_light_in_storage(
    light_manager: &mut LightManager,
    light_id: UnidirectionalLightID,
    view_transform: &Isometry3,
    unidirectional_emission: &UnidirectionalEmission,
    flags: LightFlags,
) {
    let direction = unidirectional_emission.direction.unpack();
    let camera_space_direction = view_transform.transform_unit_vector(&direction);

    let light = light_manager.unidirectional_light_mut(light_id);
    light.set_camera_space_direction(camera_space_direction.pack());
    light.set_perpendicular_illuminance(unidirectional_emission.perpendicular_illuminance);
    light.set_angular_extent(unidirectional_emission.angular_source_extent);
    light.set_flags(flags);
}

pub fn sync_unidirectional_light_with_orientation_in_storage(
    light_manager: &mut LightManager,
    light_id: UnidirectionalLightID,
    view_transform: &Isometry3,
    orientation: &UnitQuaternion,
    unidirectional_emission: &UnidirectionalEmission,
    flags: LightFlags,
) {
    let direction = unidirectional_emission.direction.unpack();
    let world_direction = orientation.rotate_unit_vector(&direction);
    let camera_space_direction = view_transform.transform_unit_vector(&world_direction);

    let light = light_manager.unidirectional_light_mut(light_id);
    light.set_camera_space_direction(camera_space_direction.pack());
    light.set_perpendicular_illuminance(unidirectional_emission.perpendicular_illuminance);
    light.set_angular_extent(unidirectional_emission.angular_source_extent);
    light.set_flags(flags);
}

pub fn sync_shadowable_unidirectional_light_in_storage(
    light_manager: &mut LightManager,
    light_id: ShadowableUnidirectionalLightID,
    view_transform: &Isometry3,
    unidirectional_emission: &ShadowableUnidirectionalEmission,
    flags: LightFlags,
) {
    let direction = unidirectional_emission.direction.unpack();
    let camera_space_direction = view_transform.transform_unit_vector(&direction);
    let light = light_manager.shadowable_unidirectional_light_mut(light_id);

    light.set_camera_space_direction(camera_space_direction.pack());
    light.set_perpendicular_illuminance(unidirectional_emission.perpendicular_illuminance);
    light.set_angular_extent(unidirectional_emission.angular_source_extent);
    light.set_flags(flags);
}

pub fn sync_shadowable_unidirectional_light_with_orientation_in_storage(
    light_manager: &mut LightManager,
    light_id: ShadowableUnidirectionalLightID,
    view_transform: &Isometry3,
    orientation: &UnitQuaternion,
    unidirectional_emission: &ShadowableUnidirectionalEmission,
    flags: LightFlags,
) {
    let direction = unidirectional_emission.direction.unpack();
    let world_direction = orientation.rotate_unit_vector(&direction);
    let camera_space_direction = view_transform.transform_unit_vector(&world_direction);

    let light = light_manager.shadowable_unidirectional_light_mut(light_id);
    light.set_camera_space_direction(camera_space_direction.pack());
    light.set_perpendicular_illuminance(unidirectional_emission.perpendicular_illuminance);
    light.set_angular_extent(unidirectional_emission.angular_source_extent);
    light.set_flags(flags);
}
