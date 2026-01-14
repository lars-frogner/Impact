module [
    entity_ids,
    setup!,
    handle_keyboard_event!,
    handle_mouse_button_event!,
]

import core.Radians
import core.UnitQuaternion
import core.UnitVector3
import core.Vector3
import core.Matrix3
import core.Point3
import core.Sphere
import pf.Command
import pf.Entity
import pf.Setup.LocalForce
import pf.Setup.CylinderMesh
import pf.Setup.SphereMesh
import pf.Setup.CapsuleMesh
import pf.Setup.DynamicRigidBodyInertialProperties
import pf.Comp.AngularVelocityControl
import pf.Control.AngularVelocityControlDirections
import pf.Setup.Parent
import pf.Setup.PerspectiveCamera
import pf.Comp.ReferenceFrame
import pf.Comp.ModelTransform
import pf.Setup.SceneGraphGroup
import pf.Setup.SphereMesh
import pf.Comp.ShadowableOmnidirectionalEmission
import pf.Setup.UniformColor
import pf.Setup.UniformEmissiveLuminance
import pf.Comp.Motion
import pf.Comp.VoxelAbsorbingCapsule
import pf.Comp.VoxelAbsorbingSphere
import pf.Input.KeyboardEvent exposing [KeyboardEvent]
import pf.Input.MouseButtonEvent exposing [MouseButtonEvent]
import InputHandling.MouseButton as MouseButtonInput
import pf.Comp.SceneEntityFlags
import pf.Physics.ContactResponseParameters
import pf.Setup.LocalForce
import pf.Setup.SphericalCollidable
import pf.Comp.LocalForceGeneratorID

entity_ids = {
    player: Entity.id("player"),
    player_body: Entity.id("player_body"),
    player_head: Entity.id("player_head"),
    laser: Entity.id("laser"),
    absorbing_sphere: Entity.id("absorbing_sphere"),
}

player_mass = 1.0
thruster_acceleration = 10.0
thruster_force = player_mass * thruster_acceleration

body_segment_length = 1.2
body_radius = 0.4

laser_range = 500.0
laser_visual_radius = 0.02
laser_absorb_radius = 1.0
laser_absorb_rate = 2000.0

absorb_sphere_visual_radius = 0.05
absorb_sphere_absorb_radius = 1.0
absorb_sphere_absorb_rate = 30.0

setup! : {} => Result {} Str
setup! = |_|
    Entity.create_with_id!(player, entity_ids.player)?
    Entity.create_with_id!(player_body, entity_ids.player_body)?
    Entity.create_with_id!(player_head, entity_ids.player_head)?
    Entity.create_with_id!(laser, entity_ids.laser)?
    Entity.create_with_id!(absorbing_sphere, entity_ids.absorbing_sphere)?

    Ok({})

handle_keyboard_event! : KeyboardEvent => Result {} Str
handle_keyboard_event! = |{ key, state }|
    force =
        when state is
            Pressed -> thruster_force
            Released -> -thruster_force
            Held ->
                return Ok({})

    force_vector =
        when key is
            Letter(letter_key) ->
                when letter_key is
                    KeyW -> Some((0, 0, force))
                    KeyS -> Some((0, 0, -force))
                    KeyD -> Some((-force, 0, 0))
                    KeyA -> Some((force, 0, 0))
                    KeyQ -> Some((0, -force, 0))
                    KeyE -> Some((0, force, 0))
                    _ -> None

            _ -> None

    when force_vector is
        Some(f) -> add_thruster_force_for_player!(f)
        None -> Ok({})

add_thruster_force_for_player! = |force|
    generator_id = Comp.LocalForceGeneratorID.get_for_entity!(entity_ids.player)?
    Command.execute!(Engine(Physics(UpdateLocalForce { generator_id, mode: Add, force })))

handle_mouse_button_event! : MouseButtonEvent => Result {} Str
handle_mouse_button_event! = |{ button, state }|
    when button is
        Left ->
            MouseButtonInput.toggle_scene_entity_active_state!(
                entity_ids.laser,
                state,
            )

        Right ->
            MouseButtonInput.toggle_scene_entity_active_state!(
                entity_ids.absorbing_sphere,
                state,
            )

        _ -> Ok({})

player =
    Entity.new_component_data
    |> Setup.SceneGraphGroup.add
    |> Comp.ReferenceFrame.add_unoriented((0.0, 0.0, -150.0))
    |> Comp.Motion.add_stationary
    |> Comp.AngularVelocityControl.add_new(
        Control.AngularVelocityControlDirections.horizontal,
    )
    |> Setup.DynamicRigidBodyInertialProperties.add_new(
        player_mass,
        Vector3.zero,
        Matrix3.diagonal(Vector3.same(1.0)),
    )
    |> Setup.SphericalCollidable.add_new(
        Dynamic,
        Sphere.new(Point3.origin, body_radius),
        Physics.ContactResponseParameters.new(0.0, 0, 0),
    )
    |> Setup.LocalForce.add_new(
        Vector3.zero,
        Point3.origin,
    )

player_body =
    Entity.new_component_data
    |> Setup.Parent.add_new(entity_ids.player)
    |> Setup.CapsuleMesh.add_new(
        body_segment_length,
        body_radius,
        128,
    )
    |> Setup.UniformColor.add((0.2, 0.8, 0.2))
    |> Comp.ReferenceFrame.add_unoriented(Point3.origin)

player_head =
    Entity.new_component_data
    |> Setup.Parent.add_new(entity_ids.player)
    |> Setup.SceneGraphGroup.add
    |> Comp.ReferenceFrame.add_new(
        (0.0, 0.5 * body_segment_length + body_radius, 1.2 * body_radius),
        UnitQuaternion.from_axis_angle(UnitVector3.y_axis, Num.pi),
    )
    |> Comp.Motion.add_stationary
    |> Comp.AngularVelocityControl.add_new(
        Control.AngularVelocityControlDirections.vertical,
    )
    |> Setup.PerspectiveCamera.add_new(
        Radians.from_degrees(70),
        0.01,
        1000,
    )

laser =
    Entity.new_component_data
    |> Setup.Parent.add_new(entity_ids.player_head)
    |> Comp.ReferenceFrame.add_new(
        (0.15, -0.3, 0.0),
        UnitQuaternion.from_axis_angle(UnitVector3.x_axis, (-Num.pi) / 2),
    )
    |> Setup.CylinderMesh.add_new(laser_range, 2 * laser_visual_radius, 16)
    |> Setup.UniformColor.add((0.9, 0.05, 0.05))
    |> Setup.UniformEmissiveLuminance.add(1e6)
    |> Comp.VoxelAbsorbingCapsule.add_new(
        Vector3.same(0),
        (0, laser_range, 0),
        laser_absorb_radius,
        laser_absorb_rate,
    )
    |> Comp.SceneEntityFlags.add(
        Comp.SceneEntityFlags.union(
            Comp.SceneEntityFlags.is_disabled,
            Comp.SceneEntityFlags.casts_no_shadows,
        ),
    )

absorbing_sphere =
    Entity.new_component_data
    |> Setup.Parent.add_new(entity_ids.player_head)
    |> Comp.ModelTransform.add_with_scale(2 * absorb_sphere_visual_radius)
    |> Comp.ReferenceFrame.add_unoriented((0, 0, -3))
    |> Setup.SphereMesh.add_new(64)
    |> Setup.UniformColor.add((0.9, 0.05, 0.05))
    |> Setup.UniformEmissiveLuminance.add(1e6)
    |> Comp.ShadowableOmnidirectionalEmission.add_new(Vector3.scale((1.0, 0.2, 0.2), 1e5), 0.2)
    |> Comp.VoxelAbsorbingSphere.add_new(
        Vector3.same(0),
        absorb_sphere_absorb_radius,
        absorb_sphere_absorb_rate,
    )
    |> Comp.SceneEntityFlags.add(Comp.SceneEntityFlags.is_disabled)
