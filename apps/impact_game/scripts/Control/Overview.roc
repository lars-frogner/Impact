module [
    rotate_camera!,
    pan_camera!,
    zoom_camera!,
]

import core.NumUtil
import core.Vector3
import core.Point3
import core.UnitQuaternion

import pf.Comp.ReferenceFrame

import pf.Entity

zoom_sensitivity = 5e-3
trackball_sensitivity = 2.0

rotate_camera! : Entity.Id, Entity.Id, F32, F32, F32, F32 => Result {} Str
rotate_camera! = |camera_entity_id, target_entity_id, ang_delta_x, ang_delta_y, ang_x, ang_y|
    camera_frame = Comp.ReferenceFrame.get_for_entity!(camera_entity_id)?
    target_frame = Comp.ReferenceFrame.get_for_entity!(target_entity_id)?

    (target_ang_x, target_ang_y) = angular_position_of_target_center(camera_frame, target_frame)

    rel_ang_x = ang_x - target_ang_x
    rel_ang_y = ang_y - target_ang_y

    prev_rel_ang_x = rel_ang_x - ang_delta_x * trackball_sensitivity
    prev_rel_ang_y = rel_ang_y - ang_delta_y * trackball_sensitivity

    dir = direction_from_cursor_angles(rel_ang_x, rel_ang_y)
    prev_dir = direction_from_cursor_angles(prev_rel_ang_x, prev_rel_ang_y)

    camera_space_rotation_axis = Vector3.cross(prev_dir, dir) |> Vector3.normalized
    rotation_angle = Num.acos(NumUtil.clamp(Vector3.dot(prev_dir, dir), -1.0, 1.0))

    rotation_axis = camera_frame.orientation |> UnitQuaternion.rotate_vector(camera_space_rotation_axis)

    # This would be the rotation of the target
    target_rotation = UnitQuaternion.from_axis_angle(rotation_axis, rotation_angle)

    camera_rotation = UnitQuaternion.inverse(target_rotation)

    position = UnitQuaternion.rotate_vector(camera_rotation, camera_frame.position)
    orientation = UnitQuaternion.mul(camera_rotation, camera_frame.orientation)

    Comp.ReferenceFrame.set_for_entity!({ position, orientation }, camera_entity_id)

angular_position_of_target_center = |camera_frame, target_frame|
    # Vector from camera to target, in world space
    target_offset_world_space = Vector3.sub(target_frame.position, camera_frame.position)

    # Convert to camera space by rotating with the inverse camera orientation
    inverse_camera_orientation = UnitQuaternion.inverse(camera_frame.orientation)
    target_offset_camera_space = UnitQuaternion.rotate_vector(inverse_camera_orientation, target_offset_world_space)

    (x, y, z) = target_offset_camera_space
    z_eps = if Num.abs(z) < 1e-6 then 1e-6 else z

    # Project to angular position on screen
    target_ang_x = -Num.atan(x / z_eps)
    target_ang_y = -Num.atan(y / z_eps)
    (target_ang_x, target_ang_y)

direction_from_cursor_angles = |ang_x, ang_y|
    Vector3.normalized((Num.tan(ang_x), Num.tan(ang_y), 1.0))

pan_camera! : Entity.Id, Entity.Id, F32, F32 => Result {} Str
pan_camera! = |camera_entity_id, target_entity_id, ang_delta_x, ang_delta_y|
    camera_frame = Comp.ReferenceFrame.get_for_entity!(camera_entity_id)?
    target_frame = Comp.ReferenceFrame.get_for_entity!(target_entity_id)?

    (view_x, view_y, _) = UnitQuaternion.to_rotation_matrix(camera_frame.orientation)

    dist = Point3.distance_between(camera_frame.position, target_frame.position)

    target_offset_x = view_x |> Vector3.scale(ang_delta_x * dist)
    target_offset_y = view_y |> Vector3.scale(ang_delta_y * dist)

    camera_offset_x = Vector3.flipped(target_offset_x)
    camera_offset_y = Vector3.flipped(target_offset_y)

    position =
        camera_frame.position
        |> Vector3.add(camera_offset_x)
        |> Vector3.add(camera_offset_y)

    Comp.ReferenceFrame.set_for_entity!({ camera_frame & position }, camera_entity_id)

zoom_camera! : Entity.Id, Entity.Id, F32 => Result {} Str
zoom_camera! = |camera_entity_id, target_entity_id, delta_y|
    camera_frame = Comp.ReferenceFrame.get_for_entity!(camera_entity_id)?
    target_frame = Comp.ReferenceFrame.get_for_entity!(target_entity_id)?

    (_, _, view_z) = UnitQuaternion.to_rotation_matrix(camera_frame.orientation)

    dist = Point3.distance_between(camera_frame.position, target_frame.position)

    target_offset_z = view_z |> Vector3.scale(delta_y * dist * zoom_sensitivity)

    camera_offset_z = Vector3.flipped(target_offset_z)

    position =
        camera_frame.position
        |> Vector3.add(camera_offset_z)

    Comp.ReferenceFrame.set_for_entity!({ camera_frame & position }, camera_entity_id)
