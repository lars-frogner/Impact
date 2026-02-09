//! Light setup.

use crate::{
    AmbientEmission, AmbientLight, AmbientLightID, LightFlags, LightManager,
    OmnidirectionalEmission, OmnidirectionalLight, OmnidirectionalLightID,
    ShadowableOmnidirectionalEmission, ShadowableOmnidirectionalLight,
    ShadowableOmnidirectionalLightID, ShadowableUnidirectionalEmission,
    ShadowableUnidirectionalLight, ShadowableUnidirectionalLightID, UnidirectionalEmission,
    UnidirectionalLight, UnidirectionalLightID,
};
use anyhow::Result;
use impact_id::EntityID;
use impact_math::{
    angle::Degrees, point::Point3, quaternion::UnitQuaternion, transform::Isometry3,
};

pub fn setup_ambient_light(
    light_manager: &mut LightManager,
    entity_id: EntityID,
    ambient_emission: &AmbientEmission,
) -> Result<()> {
    let ambient_light = AmbientLight::new(crate::compute_luminance_for_uniform_illuminance(
        &ambient_emission.illuminance,
    ));

    let light_id = AmbientLightID::from_entity_id(entity_id);
    light_manager.add_ambient_light(light_id, ambient_light)
}

pub fn setup_omnidirectional_light(
    light_manager: &mut LightManager,
    entity_id: EntityID,
    view_transform: &Isometry3,
    position: &Point3,
    omnidirectional_emission: &OmnidirectionalEmission,
    flags: LightFlags,
) -> Result<()> {
    let position = view_transform.transform_point(position);
    let omnidirectional_light = OmnidirectionalLight::new(
        position.compact(),
        omnidirectional_emission.luminous_intensity,
        f32::max(omnidirectional_emission.source_extent, 0.0),
        flags,
    );

    let light_id = OmnidirectionalLightID::from_entity_id(entity_id);
    light_manager.add_omnidirectional_light(light_id, omnidirectional_light)
}

pub fn setup_shadowable_omnidirectional_light(
    light_manager: &mut LightManager,
    entity_id: EntityID,
    view_transform: &Isometry3,
    position: &Point3,
    omnidirectional_emission: &ShadowableOmnidirectionalEmission,
    flags: LightFlags,
) -> Result<()> {
    let position = view_transform.transform_point(position);
    let omnidirectional_light = ShadowableOmnidirectionalLight::new(
        position.compact(),
        omnidirectional_emission.luminous_intensity,
        f32::max(omnidirectional_emission.source_extent, 0.0),
        flags,
    );

    let light_id = ShadowableOmnidirectionalLightID::from_entity_id(entity_id);
    light_manager.add_shadowable_omnidirectional_light(light_id, omnidirectional_light)
}

pub fn setup_unidirectional_light(
    light_manager: &mut LightManager,
    entity_id: EntityID,
    view_transform: &Isometry3,
    unidirectional_emission: &UnidirectionalEmission,
    flags: LightFlags,
) -> Result<()> {
    let direction = unidirectional_emission.direction.aligned();
    let camera_space_direction = view_transform.transform_unit_vector(&direction);

    let unidirectional_light = UnidirectionalLight::new(
        camera_space_direction.compact(),
        unidirectional_emission.perpendicular_illuminance,
        Degrees(f32::max(
            unidirectional_emission.angular_source_extent.0,
            0.0,
        )),
        flags,
    );

    let light_id = UnidirectionalLightID::from_entity_id(entity_id);
    light_manager.add_unidirectional_light(light_id, unidirectional_light)
}

pub fn setup_shadowable_unidirectional_light(
    light_manager: &mut LightManager,
    entity_id: EntityID,
    view_transform: &Isometry3,
    unidirectional_emission: &ShadowableUnidirectionalEmission,
    flags: LightFlags,
) -> Result<()> {
    let direction = unidirectional_emission.direction.aligned();
    let camera_space_direction = view_transform.transform_unit_vector(&direction);

    let unidirectional_light = ShadowableUnidirectionalLight::new(
        camera_space_direction.compact(),
        unidirectional_emission.perpendicular_illuminance,
        Degrees(f32::max(
            unidirectional_emission.angular_source_extent.0,
            0.0,
        )),
        flags,
    );

    let light_id = ShadowableUnidirectionalLightID::from_entity_id(entity_id);
    light_manager.add_shadowable_unidirectional_light(light_id, unidirectional_light)
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
    light.set_camera_space_position(view_transform.transform_point(position).compact());
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
    light.set_camera_space_position(view_transform.transform_point(position).compact());
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
    let direction = unidirectional_emission.direction.aligned();
    let camera_space_direction = view_transform.transform_unit_vector(&direction);

    let light = light_manager.unidirectional_light_mut(light_id);
    light.set_camera_space_direction(camera_space_direction.compact());
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
    let direction = unidirectional_emission.direction.aligned();
    let world_direction = orientation.rotate_unit_vector(&direction);
    let camera_space_direction = view_transform.transform_unit_vector(&world_direction);

    let light = light_manager.unidirectional_light_mut(light_id);
    light.set_camera_space_direction(camera_space_direction.compact());
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
    let direction = unidirectional_emission.direction.aligned();
    let camera_space_direction = view_transform.transform_unit_vector(&direction);
    let light = light_manager.shadowable_unidirectional_light_mut(light_id);

    light.set_camera_space_direction(camera_space_direction.compact());
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
    let direction = unidirectional_emission.direction.aligned();
    let world_direction = orientation.rotate_unit_vector(&direction);
    let camera_space_direction = view_transform.transform_unit_vector(&world_direction);

    let light = light_manager.shadowable_unidirectional_light_mut(light_id);
    light.set_camera_space_direction(camera_space_direction.compact());
    light.set_perpendicular_illuminance(unidirectional_emission.perpendicular_illuminance);
    light.set_angular_extent(unidirectional_emission.angular_source_extent);
    light.set_flags(flags);
}
