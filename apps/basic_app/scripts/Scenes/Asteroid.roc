module [
    entity_ids,
    setup!,
    handle_mouse_button_event!,
]

import core.Radians
import core.Plane
import core.UnitQuaternion
import core.UnitVector3 exposing [y_axis]
import core.Vector3
import pf.Command
import pf.Entity
import pf.Skybox
import pf.Comp.AmbientEmission
import pf.Setup.CylinderMesh
import pf.Setup.GradientNoiseVoxelTypes
import pf.Setup.SameVoxelType
import pf.Comp.ControlledVelocity
import pf.Setup.MultifractalNoiseSDFModification
import pf.Comp.ControlledAngularVelocity
import pf.Setup.Parent
import pf.Setup.PerspectiveCamera
import pf.Setup.RectangleMesh
import pf.Comp.ReferenceFrame
import pf.Comp.ModelTransform
import pf.Setup.SceneGraphGroup
import pf.Setup.SphereMesh
import pf.Comp.ShadowableOmnidirectionalEmission
import pf.Comp.ShadowableUnidirectionalEmission
import pf.Setup.UniformColor
import pf.Setup.UniformEmissiveLuminance
import pf.Setup.UniformRoughness
import pf.Setup.UniformSpecularReflectance
import pf.Comp.Motion
import pf.Comp.VoxelAbsorbingCapsule
import pf.Comp.VoxelAbsorbingSphere
import pf.Setup.VoxelSphereUnion
import pf.Setup.VoxelBox
import pf.Setup.DynamicVoxels
import pf.Input.MouseButtonEvent exposing [MouseButtonEvent]
import InputHandling.MouseButton as MouseButtonInput
import pf.Physics.AngularVelocity as AngularVelocity
import pf.Texture.TextureID
import pf.Comp.SceneEntityFlags
import pf.Setup.PlanarCollidable
import pf.Setup.VoxelCollidable
import pf.Physics.ContactResponseParameters
import pf.Setup.ConstantAcceleration
import pf.Setup.LocalForce

entity_ids = {
    player: Entity.id("player"),
    camera: Entity.id("camera"),
    laser: Entity.id("laser"),
    absorbing_sphere: Entity.id("absorbing_sphere"),
    ground: Entity.id("ground"),
    asteroid: Entity.id("asteroid"),
    ambient_light: Entity.id("ambient_light"),
    omnidirectional_light: Entity.id("omnidirectional_light"),
    unidirectional_light: Entity.id("unidirectional_light"),
}

setup! : {} => Result {} Str
setup! = |_|
    # Command.execute!(Engine(Scene(SetSkybox(Skybox.new(skybox, 2e3)))))?

    Entity.create_with_id!(player, entity_ids.player)?
    Entity.create_with_id!(camera, entity_ids.camera)?
    Entity.create_with_id!(laser, entity_ids.laser)?
    Entity.create_with_id!(absorbing_sphere, entity_ids.absorbing_sphere)?
    Entity.create_with_id!(ground, entity_ids.ground)?
    Entity.create_with_id!(asteroid, entity_ids.asteroid)?
    Entity.create_with_id!(ambient_light, entity_ids.ambient_light)?
    Entity.create_with_id!(omnidirectional_light, entity_ids.omnidirectional_light)?
    Entity.create_with_id!(unidirectional_light, entity_ids.unidirectional_light)?

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

skybox = Texture.TextureID.from_name("space_skybox")

player =
    Entity.new_component_data
    |> Comp.ReferenceFrame.add_new(
        (0.0, 0.0, 0.0),
        UnitQuaternion.from_axis_angle(UnitVector3.y_axis, Num.pi),
    )
    |> Comp.Motion.add_stationary
    |> Comp.ControlledVelocity.add_new
    |> Comp.ControlledAngularVelocity.add_new
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

ground =
    Entity.new_component_data
    |> Setup.RectangleMesh.add_unit_square
    |> Comp.ModelTransform.add_with_scale(500)
    |> Comp.ReferenceFrame.add_unoriented((0, -20, 0))
    |> Comp.Motion.add_stationary
    |> Setup.UniformColor.add((1, 1, 1))
    |> Setup.UniformSpecularReflectance.add(0.01)
    |> Setup.UniformRoughness.add(0.5)
    |> Setup.PlanarCollidable.add_new(
        Static,
        Plane.new(y_axis, 0),
        Physics.ContactResponseParameters.new(0.2, 0.7, 0.5),
    )

asteroid =
    Entity.new_component_data
    |> Comp.ReferenceFrame.add_unoriented((0, 0, 30))
    |> Setup.VoxelSphereUnion.add_new(0.25, 10, 10, (20, 0, 0), 5.0)
    # |> Setup.VoxelBox.add_new(0.25, 31, 15, 15)
    |> Setup.GradientNoiseVoxelTypes.add_new(["Ground", "Rock", "Metal"], 6e-2, 1, 1)
    # |> Setup.SameVoxelType.add_new("Default")
    |> Setup.MultifractalNoiseSDFModification.add_new(8, 0.02, 2.0, 0.6, 4.0, 0)
    |> Comp.Motion.add_angular(AngularVelocity.new(UnitVector3.y_axis, Radians.from_degrees(0)))
    |> Setup.DynamicVoxels.add
    |> Setup.VoxelCollidable.add_new(
        Dynamic,
        Physics.ContactResponseParameters.new(0.2, 0.7, 0.5),
    )
    # |> Setup.ConstantAcceleration.add_earth
    |> Setup.LocalForce.add_new((0.1, 0.1, 0.1), (4, 4, 4))

ambient_light =
    Entity.new_component_data
    |> Comp.AmbientEmission.add_new(Vector3.same(1000))

omnidirectional_light =
    Entity.new_component_data
    |> Setup.SphereMesh.add_new(25)
    |> Comp.ModelTransform.add_with_scale(0.7)
    |> Comp.ReferenceFrame.add_unoriented((0, 15, 2))
    |> Setup.UniformColor.add((1, 1, 1))
    |> Setup.UniformEmissiveLuminance.add(1e6)
    |> Comp.ShadowableOmnidirectionalEmission.add_new(
        Vector3.same(2e7),
        0.7,
    )

unidirectional_light =
    Entity.new_component_data
    |> Comp.ShadowableUnidirectionalEmission.add_new(
        Vector3.same(20000),
        UnitVector3.from((0.0, -1.0, 0.0)),
        2.0,
    )
