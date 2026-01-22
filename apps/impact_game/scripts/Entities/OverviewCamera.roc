module [
    entity_ids,
    camera,
    spawn!,
]

import core.NumUtil
import core.Radians
import core.Point3
import core.UnitVector3
import core.UnitQuaternion

import pf.Entity

import pf.Setup.PerspectiveCamera
import pf.Comp.ReferenceFrame

entity_ids = {
    camera: Entity.id("overview_camera"),
}

camera = {
    field_of_view: 70,
    focus_position: Point3.origin,
}

spawn! : F32 => Result {} Str
spawn! = |radius_to_cover|
    camera_ent = construct_entity(radius_to_cover)
    Entity.create_with_id!(camera_ent, entity_ids.camera)?
    Ok({})

construct_entity : F32 -> Entity.ComponentData
construct_entity = |radius_to_cover|
    height = height_to_cover_radius(radius_to_cover)
    (near_distance, far_distance) = near_and_far_distance_for_cover_radius(radius_to_cover)

    Entity.new_component_data
    |> Comp.ReferenceFrame.add_new(
        (0.0, height, 0.0),
        UnitQuaternion.mul(
            UnitQuaternion.from_axis_angle(UnitVector3.unit_y, Num.pi),
            UnitQuaternion.from_axis_angle(UnitVector3.unit_x, (-Num.pi) / 2),
        ),
    )
    |> Setup.PerspectiveCamera.add_new(
        Radians.from_degrees(camera.field_of_view),
        near_distance,
        far_distance,
    )

height_to_cover_radius = |radius_to_cover|
    radius_to_cover / Num.tan(0.5 * Radians.from_degrees(camera.field_of_view))

near_and_far_distance_for_cover_radius = |radius_to_cover|
    far = NumUtil.clamp(10 * radius_to_cover, 1e4, 1e6)
    near = NumUtil.clamp(1e-4 * far, 1e-2, far)
    (near, far)
