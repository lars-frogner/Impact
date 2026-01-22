module [
    rotate_camera!,
    pan_camera!,
    zoom_camera!,
]

import core.NumUtil
import core.Vector3
import core.Point3 exposing [Point3]
import core.UnitQuaternion

import pf.Comp.ReferenceFrame

import pf.Entity

zoom_sensitivity = 5e-3
trackball_sensitivity = 2.0

rotate_camera! : Entity.Id, Point3, F32, F32, F32, F32 => Result {} Str
rotate_camera! = |camera_entity_id, focus_position, ang_delta_x, ang_delta_y, ang_x, ang_y|
    camera_frame = Comp.ReferenceFrame.get_for_entity!(camera_entity_id)?

    (focus_ang_x, focus_ang_y) = angular_position_of_focus(camera_frame, focus_position)

    rel_ang_x = ang_x - focus_ang_x
    rel_ang_y = ang_y - focus_ang_y

    prev_rel_ang_x = rel_ang_x - ang_delta_x * trackball_sensitivity
    prev_rel_ang_y = rel_ang_y - ang_delta_y * trackball_sensitivity

    dir = direction_from_cursor_angles(rel_ang_x, rel_ang_y)
    prev_dir = direction_from_cursor_angles(prev_rel_ang_x, prev_rel_ang_y)

    camera_space_rotation_axis = Vector3.cross(prev_dir, dir) |> Vector3.normalized
    rotation_angle = -Num.acos(NumUtil.clamp(Vector3.dot(prev_dir, dir), -1.0, 1.0))

    rotation_axis = camera_frame.orientation |> UnitQuaternion.rotate_vector(camera_space_rotation_axis)

    camera_rotation = UnitQuaternion.from_axis_angle(rotation_axis, rotation_angle)

    position = UnitQuaternion.rotate_vector(camera_rotation, camera_frame.position)
    orientation = UnitQuaternion.mul(camera_rotation, camera_frame.orientation)

    Comp.ReferenceFrame.set_for_entity!({ position, orientation }, camera_entity_id)

angular_position_of_focus = |camera_frame, focus_position|
    # Vector from camera to focus position, in world space
    focus_offset_world_space = Vector3.sub(focus_position, camera_frame.position)

    # Convert to camera space by rotating with the inverse camera orientation
    inverse_camera_orientation = UnitQuaternion.inverse(camera_frame.orientation)
    focus_offset_camera_space = UnitQuaternion.rotate_vector(inverse_camera_orientation, focus_offset_world_space)

    (x, y, z) = focus_offset_camera_space
    z_eps = if Num.abs(z) < 1e-6 then 1e-6 else z

    # Project to angular position on screen
    target_ang_x = -Num.atan(x / z_eps)
    target_ang_y = -Num.atan(y / z_eps)
    (target_ang_x, target_ang_y)

direction_from_cursor_angles = |ang_x, ang_y|
    Vector3.normalized((Num.tan(ang_x), Num.tan(ang_y), 1.0))

pan_camera! : Entity.Id, Point3, F32, F32 => Result {} Str
pan_camera! = |camera_entity_id, focus_position, ang_delta_x, ang_delta_y|
    camera_frame = Comp.ReferenceFrame.get_for_entity!(camera_entity_id)?

    (view_x, view_y, _) = UnitQuaternion.to_rotation_matrix(camera_frame.orientation)

    dist = Point3.distance_between(camera_frame.position, focus_position)

    target_offset_x = view_x |> Vector3.scale(ang_delta_x * dist)
    target_offset_y = view_y |> Vector3.scale(ang_delta_y * dist)

    camera_offset_x = Vector3.flipped(target_offset_x)
    camera_offset_y = Vector3.flipped(target_offset_y)

    position =
        camera_frame.position
        |> Vector3.add(camera_offset_x)
        |> Vector3.add(camera_offset_y)

    Comp.ReferenceFrame.set_for_entity!({ camera_frame & position }, camera_entity_id)

zoom_camera! : Entity.Id, Point3, F32 => Result {} Str
zoom_camera! = |camera_entity_id, focus_position, delta_y|
    camera_frame = Comp.ReferenceFrame.get_for_entity!(camera_entity_id)?

    (_, _, view_z) = UnitQuaternion.to_rotation_matrix(camera_frame.orientation)

    dist = Point3.distance_between(camera_frame.position, focus_position)

    target_offset_z = view_z |> Vector3.scale(delta_y * dist * zoom_sensitivity)

    camera_offset_z = Vector3.flipped(target_offset_z)

    position =
        camera_frame.position
        |> Vector3.add(camera_offset_z)

    Comp.ReferenceFrame.set_for_entity!({ camera_frame & position }, camera_entity_id)
