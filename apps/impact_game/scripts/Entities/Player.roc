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
import pf.Setup.DynamicRigidBodyInertialProperties
import pf.Comp.ControlledAngularVelocity
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
    camera: Entity.id("camera"),
    laser: Entity.id("laser"),
    absorbing_sphere: Entity.id("absorbing_sphere"),
}

player_mass = 1.0
thruster_acceleration = 5.0
thruster_force = player_mass * thruster_acceleration

setup! : {} => Result {} Str
setup! = |_|
    Entity.create_with_id!(player, entity_ids.player)?
    Entity.create_with_id!(camera, entity_ids.camera)?
    Entity.create_with_id!(laser, entity_ids.laser)?
    Entity.create_with_id!(absorbing_sphere, entity_ids.absorbing_sphere)?

    Ok({})

handle_keyboard_event! : KeyboardEvent => Result {} Str
handle_keyboard_event! = |{ key, state }|
    force =
        when state is
            Pressed -> thruster_force
            Released -> 0

    force_vector =
        when key is
            Letter(letter_key) ->
                when letter_key is
                    KeyW -> Some((0, 0, -force))
                    KeyS -> Some((0, 0, force))
                    KeyA -> Some((-force, 0, 0))
                    KeyD -> Some((force, 0, 0))
                    KeyQ -> Some((0, -force, 0))
                    KeyE -> Some((0, force, 0))
                    _ -> None

            _ -> None

    when force_vector is
        Some(f) -> set_thruster_force_for_player!(f)
        None -> Ok({})

set_thruster_force_for_player! = |force|
    generator_id = Comp.LocalForceGeneratorID.get_for_entity!(entity_ids.player)?
    Command.execute!(Engine(Physics(SetLocalForce { generator_id, force })))

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
    |> Comp.ReferenceFrame.add_new(
        (0.0, 0.0, -150.0),
        UnitQuaternion.from_axis_angle(UnitVector3.y_axis, Num.pi),
    )
    |> Comp.Motion.add_stationary
    |> Setup.LocalForce.add_new(Vector3.zero, Vector3.zero)
    |> Comp.ControlledAngularVelocity.add_new
    |> Setup.DynamicRigidBodyInertialProperties.add_new(player_mass, Vector3.zero, Matrix3.diagonal(Vector3.same(1.0)))
    |> Setup.SphericalCollidable.add_new(
        Dynamic,
        Sphere.new(Point3.origin, 1.0),
        Physics.ContactResponseParameters.new(0.0, 0, 0),
    )
    |> Setup.SceneGraphGroup.add

camera =
    Entity.new_component_data
    |> Setup.Parent.add_new(entity_ids.player)
    |> Setup.PerspectiveCamera.add_new(Radians.from_degrees(70), 0.01, 1000)

laser =
    Entity.new_component_data
    |> Setup.Parent.add_new(entity_ids.player)
    |> Comp.ReferenceFrame.add_new(
        (0.15, -0.3, 0.0),
        UnitQuaternion.from_axis_angle(UnitVector3.x_axis, (-Num.pi) / 2),
    )
    |> Setup.CylinderMesh.add_new(100, 0.02, 16)
    |> Setup.UniformColor.add((0.9, 0.05, 0.05))
    |> Setup.UniformEmissiveLuminance.add(1e6)
    |> Comp.VoxelAbsorbingCapsule.add_new(Vector3.same(0), (0, 100, 0), 0.3, 200)
    |> Comp.SceneEntityFlags.add(
        Comp.SceneEntityFlags.union(
            Comp.SceneEntityFlags.is_disabled,
            Comp.SceneEntityFlags.casts_no_shadows,
        ),
    )

absorbing_sphere =
    Entity.new_component_data
    |> Setup.Parent.add_new(entity_ids.player)
    |> Comp.ModelTransform.add_with_scale(0.1)
    |> Comp.ReferenceFrame.add_unoriented((0, 0, -3))
    |> Setup.SphereMesh.add_new(64)
    |> Setup.UniformColor.add((0.9, 0.05, 0.05))
    |> Setup.UniformEmissiveLuminance.add(1e6)
    |> Comp.ShadowableOmnidirectionalEmission.add_new(Vector3.scale((1.0, 0.2, 0.2), 1e5), 0.2)
    |> Comp.VoxelAbsorbingSphere.add_new(Vector3.same(0), 1, 15)
    |> Comp.SceneEntityFlags.add(Comp.SceneEntityFlags.is_disabled)
