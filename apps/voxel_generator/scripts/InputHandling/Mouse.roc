module [
    handle_button_event!,
    handle_drag_event!,
    handle_scroll_event!,
]

import core.NumUtil
import core.Vector3
import core.Point3
import core.UnitQuaternion
import pf.Stdout
import pf.Comp.ReferenceFrame
import pf.Input.MouseButtonEvent exposing [MouseButtonEvent]
import pf.Input.MouseDragEvent exposing [MouseDragEvent]
import pf.Input.MouseScrollEvent exposing [MouseScrollEvent]
import pf.Input.MouseButtonSet as Buttons
import Scene exposing [entity_ids]

zoom_sensitivity = 5e-3
trackball_sensitivity = 2.0

handle_button_event! : MouseButtonEvent => Result {} Str
handle_button_event! = |_event|
    Ok({})

handle_drag_event! : MouseDragEvent => Result {} Str
handle_drag_event! = |event|
    if Buttons.contains(event.pressed, Buttons.left) then
        rotate_object!(event.ang_delta_x, event.ang_delta_y, event.cursor.ang_x, event.cursor.ang_y)
    else if Buttons.contains(event.pressed, Buttons.right) then
        pan_object!(event.ang_delta_x, event.ang_delta_y)
    else
        Ok({})

handle_scroll_event! : MouseScrollEvent => Result {} Str
handle_scroll_event! = |event|
    zoom_object!(event.delta_y)

rotate_object! = |ang_delta_x, ang_delta_y, ang_x, ang_y|
    camera_frame = Comp.ReferenceFrame.get_for_entity!(entity_ids.camera)?
    object_frame = Comp.ReferenceFrame.get_for_entity!(entity_ids.object)?

    prev_ang_x = ang_x - ang_delta_x * trackball_sensitivity
    prev_ang_y = ang_y - ang_delta_y * trackball_sensitivity

    dir = direction_from_cursor_angles(ang_x, ang_y)
    prev_dir = direction_from_cursor_angles(prev_ang_x, prev_ang_y)

    camera_space_rotation_axis = Vector3.cross(prev_dir, dir) |> Vector3.normalize
    rotation_angle = Num.acos(NumUtil.clamp(Vector3.dot(prev_dir, dir), -1.0, 1.0))

    rotation_axis = camera_frame.orientation |> UnitQuaternion.rotate_vector(camera_space_rotation_axis)

    rotation = UnitQuaternion.from_axis_angle(rotation_axis, rotation_angle)
    orientation = UnitQuaternion.mul(rotation, object_frame.orientation)

    Comp.ReferenceFrame.set_for_entity!({ object_frame & orientation }, entity_ids.object)

direction_from_cursor_angles = |ang_x, ang_y|
    Vector3.normalize((Num.tan(ang_x), Num.tan(ang_y), 1.0))

pan_object! = |ang_delta_x, ang_delta_y|
    camera_frame = Comp.ReferenceFrame.get_for_entity!(entity_ids.camera)?
    object_frame = Comp.ReferenceFrame.get_for_entity!(entity_ids.object)?

    (view_x, view_y, _) = UnitQuaternion.to_rotation_matrix(camera_frame.orientation)

    dist = Point3.distance(camera_frame.position, object_frame.position)

    offset_x = view_x |> Vector3.scale(ang_delta_x * dist)
    offset_y = view_y |> Vector3.scale(ang_delta_y * dist)

    position =
        object_frame.position
        |> Vector3.add(offset_x)
        |> Vector3.add(offset_y)

    Comp.ReferenceFrame.set_for_entity!({ object_frame & position }, entity_ids.object)

zoom_object! = |delta_y|
    camera_frame = Comp.ReferenceFrame.get_for_entity!(entity_ids.camera)?
    object_frame = Comp.ReferenceFrame.get_for_entity!(entity_ids.object)?

    (_, _, view_z) = UnitQuaternion.to_rotation_matrix(camera_frame.orientation)

    dist = Point3.distance(camera_frame.position, object_frame.position)

    offset_z = view_z |> Vector3.scale(delta_y * dist * zoom_sensitivity)

    position =
        object_frame.position
        |> Vector3.add(offset_z)

    Comp.ReferenceFrame.set_for_entity!({ object_frame & position }, entity_ids.object)

# rotate_object_arcball! = |ang_delta_x, ang_delta_y|
#    camera_frame = Comp.ReferenceFrame.get_for_entity!(entity_ids.camera)?
#    object_frame = Comp.ReferenceFrame.get_for_entity!(entity_ids.object)?

#    (view_x, view_y, view_z) = UnitQuaternion.to_rotation_matrix(camera_frame.orientation)

#    dist = Point3.distance(camera_frame.position, object_frame.position)

#    offset_x = view_x |> Vector3.scale(ang_delta_x * dist)
#    offset_y = view_y |> Vector3.scale(ang_delta_y * dist)
#    offset_z = view_z |> Vector3.scale(dist)

#    start_direction = offset_z |> Vector3.normalize
#    end_direction = offset_z |> Vector3.add(offset_x) |> Vector3.add(offset_y) |> Vector3.normalize

#    direction_cross = Vector3.cross(start_direction, end_direction)
#    direction_dot = Vector3.dot(start_direction, end_direction)

#    rotation_axis = direction_cross |> Vector3.normalize
#    rotation_angle = Num.acos(direction_dot)

#    rotation = UnitQuaternion.from_axis_angle(rotation_axis, rotation_angle)
#    orientation = rotation |> UnitQuaternion.mul(object_frame.orientation)

#    Comp.ReferenceFrame.set_for_entity!({ object_frame & orientation }, entity_ids.object)

# rotate_object_turntable! = |ang_delta_x, ang_delta_y|
#    camera_frame = Comp.ReferenceFrame.get_for_entity!(entity_ids.camera)?
#    object_frame = Comp.ReferenceFrame.get_for_entity!(entity_ids.object)?

#    (view_x, view_y, _) = UnitQuaternion.to_rotation_matrix(camera_frame.orientation)

#    sensitivity = 2.0

#    yaw = UnitQuaternion.from_axis_angle(view_y, ang_delta_x * sensitivity)
#    pitch = UnitQuaternion.from_axis_angle(view_x, -ang_delta_y * sensitivity)
#    rotation = UnitQuaternion.mul(yaw, pitch)

#    orientation = UnitQuaternion.mul(rotation, object_frame.orientation)

#    Comp.ReferenceFrame.set_for_entity!({ object_frame & orientation }, entity_ids.object)
