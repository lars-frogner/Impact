module [
    entity_ids,
    camera,
    spawn!,
    launch_projectile!,
]

import core.Radians
import core.UnitQuaternion exposing [UnitQuaternion]
import core.UnitVector3
import core.Point3 exposing [Point3]

import pf.Entity

import pf.Comp.AngularVelocityControl
import pf.Comp.VelocityControl
import pf.Setup.PerspectiveCamera
import pf.Comp.ReferenceFrame
import pf.Comp.CanBeParent
import pf.Comp.Motion
import pf.Lookup.LauncherLaunchSpeed

import Entities.Tools as Tools

FreeCameraEntities : {
    camera : Entity.ComponentData,
}

entity_ids = {
    camera: Entity.id("free_camera"),
    tools: {
        laser: Entity.id("free_camera_laser"),
        absorber: Entity.id("free_camera_absorber"),
    },
}

camera = {
    field_of_view: 70,
    near_distance: 0.01,
    view_distance: 1e4,
}

spawn! : Point3, UnitQuaternion => Result {} Str
spawn! = |position, orientation|
    ents = construct_entities(position, orientation)

    Entity.create_with_id!(ents.camera, entity_ids.camera)?

    Tools.spawn!(entity_ids.tools, entity_ids.camera)?

    Ok({})

launch_projectile! = |_|
    frame = Comp.ReferenceFrame.get_for_entity!(entity_ids.camera)?
    motion = Comp.Motion.get_for_entity!(entity_ids.camera)?

    launch_speed = Lookup.LauncherLaunchSpeed.get!({})?.speed

    _ = Tools.spawn_projectile!(
        entity_ids.camera,
        frame.position,
        motion.linear_velocity,
        UnitQuaternion.rotate_vector(frame.orientation, UnitVector3.neg_unit_z),
        launch_speed,
    )?

    Ok({})

construct_entities : Point3, UnitQuaternion -> FreeCameraEntities
construct_entities = |position, orientation|
    camera_ent =
        Entity.new_component_data
        |> Comp.CanBeParent.add
        |> Comp.ReferenceFrame.add_new(
            position,
            UnitQuaternion.mul(
                orientation,
                UnitQuaternion.from_axis_angle(UnitVector3.unit_y, Num.pi),
            ),
        )
        |> Comp.Motion.add_stationary
        |> Comp.VelocityControl.add
        |> Comp.AngularVelocityControl.add_all_directions
        |> Setup.PerspectiveCamera.add_new(
            Radians.from_degrees(camera.field_of_view),
            camera.near_distance,
            camera.view_distance,
        )

    {
        camera: camera_ent,
    }
