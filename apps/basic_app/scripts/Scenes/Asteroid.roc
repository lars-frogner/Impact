module [
    entity_ids,
    setup!,
    handle_mouse_button_event!,
]

import core.Radians
import core.UnitQuaternion
import core.UnitVector3
import core.Vector3
import pf.Command
import pf.Entity
import pf.Skybox
import pf.Comp.AmbientEmission
import pf.Comp.CylinderMesh
import pf.Comp.GradientNoiseVoxelTypes
import pf.Comp.MotionControl
import pf.Comp.MultifractalNoiseModification
import pf.Comp.OrientationControl
import pf.Comp.Parent
import pf.Comp.PerspectiveCamera
import pf.Comp.RectangleMesh
import pf.Comp.ReferenceFrame
import pf.Comp.SceneEntityFlags
import pf.Comp.SceneGraphGroup
import pf.Comp.SphereMesh
import pf.Comp.ShadowableOmnidirectionalEmission
import pf.Comp.UniformColor
import pf.Comp.UniformEmissiveLuminance
import pf.Comp.UniformRoughness
import pf.Comp.UniformSpecularReflectance
import pf.Comp.Velocity
import pf.Comp.VoxelAbsorbingCapsule
import pf.Comp.VoxelAbsorbingSphere
import pf.Comp.VoxelSphereUnion
import pf.Input.MouseButtonEvent exposing [MouseButtonEvent]
import InputHandling.MouseButton as MouseButtonInput
import pf.Physics.AngularVelocity as AngularVelocity
import pf.Rendering.TextureID
import pf.Scene.SceneEntityFlags

entity_ids = {
    player: Entity.id("player"),
    camera: Entity.id("camera"),
    laser: Entity.id("laser"),
    absorbing_sphere: Entity.id("absorbing_sphere"),
    ground: Entity.id("ground"),
    asteroid: Entity.id("asteroid"),
    ambient_light: Entity.id("ambient_light"),
    omnidirectional_light: Entity.id("omnidirectional_light"),
}

setup! : {} => Result {} Str
setup! = |_|
    Command.execute!(Scene(SetSkybox(Skybox.new(skybox, 2e3))))?

    Entity.create_with_id!(entity_ids.player, player)?
    Entity.create_with_id!(entity_ids.camera, camera)?
    Entity.create_with_id!(entity_ids.laser, laser)?
    Entity.create_with_id!(entity_ids.absorbing_sphere, absorbing_sphere)?
    Entity.create_with_id!(entity_ids.ground, ground)?
    Entity.create_with_id!(entity_ids.asteroid, asteroid)?
    Entity.create_with_id!(entity_ids.ambient_light, ambient_light)?
    Entity.create_with_id!(entity_ids.omnidirectional_light, omnidirectional_light)?

    Ok({})

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

skybox = Rendering.TextureID.from_name("space_skybox")

player =
    Entity.new
    |> Comp.ReferenceFrame.add_unscaled(
        (0.0, 0.0, 0.0),
        UnitQuaternion.from_axis_angle(UnitVector3.y_axis, Num.pi),
    )
    |> Comp.Velocity.add_stationary
    |> Comp.MotionControl.add_new
    |> Comp.OrientationControl.add_new
    |> Comp.SceneGraphGroup.add

camera =
    Entity.new
    |> Comp.Parent.add_new(entity_ids.player)
    |> Comp.PerspectiveCamera.add_new(Radians.from_degrees(70), 0.01, 1000)

laser =
    Entity.new
    |> Comp.Parent.add_new(entity_ids.player)
    |> Comp.ReferenceFrame.add_unscaled(
        (0.15, -0.3, 0.0),
        UnitQuaternion.from_axis_angle(UnitVector3.x_axis, (-Num.pi) / 2),
    )
    |> Comp.CylinderMesh.add_new(100, 0.02, 16)
    |> Comp.UniformColor.add((0.9, 0.05, 0.05))
    |> Comp.UniformEmissiveLuminance.add(1e6)
    |> Comp.VoxelAbsorbingCapsule.add_new(Vector3.same(0), (0, 100, 0), 0.3, 200)
    |> Comp.SceneEntityFlags.add(
        Scene.SceneEntityFlags.union(
            Scene.SceneEntityFlags.is_disabled,
            Scene.SceneEntityFlags.casts_no_shadows,
        ),
    )

absorbing_sphere =
    Entity.new
    |> Comp.Parent.add_new(entity_ids.player)
    |> Comp.ReferenceFrame.add_unoriented_scaled((0, 0, -3), 0.1)
    |> Comp.SphereMesh.add_new(64)
    |> Comp.UniformColor.add((0.9, 0.05, 0.05))
    |> Comp.UniformEmissiveLuminance.add(1e6)
    |> Comp.ShadowableOmnidirectionalEmission.add_new(Vector3.scale((1.0, 0.2, 0.2), 1e5), 0.2)
    |> Comp.VoxelAbsorbingSphere.add_new(Vector3.same(0), 10, 15)
    |> Comp.SceneEntityFlags.add(Scene.SceneEntityFlags.is_disabled)

ground =
    Entity.new
    |> Comp.RectangleMesh.add_unit_square
    |> Comp.ReferenceFrame.add_unoriented_scaled((0, -20, 0), 500)
    |> Comp.UniformColor.add((1, 1, 1))
    |> Comp.UniformSpecularReflectance.add(0.01)
    |> Comp.UniformRoughness.add(0.5)

asteroid =
    Entity.new
    |> Comp.ReferenceFrame.add_unoriented((0, 0, 30))
    |> Comp.VoxelSphereUnion.add_new(0.25, 10, 10, (20, 0, 0), 5.0)
    |> Comp.GradientNoiseVoxelTypes.add_new(["Ground", "Rock", "Metal"], 6e-2, 1, 1)
    |> Comp.MultifractalNoiseModification.add_new(8, 0.02, 2.0, 0.6, 4.0, 0)
    |> Comp.Velocity.add_angular(AngularVelocity.new(UnitVector3.y_axis, Radians.from_degrees(10)))

ambient_light =
    Entity.new
    |> Comp.AmbientEmission.add_new(Vector3.same(1000))

omnidirectional_light =
    Entity.new
    |> Comp.SphereMesh.add_new(25)
    |> Comp.ReferenceFrame.add_unoriented_scaled((0, 15, 2), 0.7)
    |> Comp.UniformColor.add((1, 1, 1))
    |> Comp.UniformEmissiveLuminance.add(1e6)
    |> Comp.ShadowableOmnidirectionalEmission.add_new(
        Vector3.same(2e7),
        0.7,
    )
