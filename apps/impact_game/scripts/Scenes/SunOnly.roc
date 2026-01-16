module [
    entity_ids,
    setup!,
    handle_keyboard_event!,
    handle_mouse_button_event!,
]

import core.Radians
import core.UnitVector3
import core.Vector3
import pf.Command
import pf.Entity
import pf.Skybox
import pf.Comp.AmbientEmission
import pf.Setup.VoxelSphere
import pf.Setup.MultifractalNoiseSDFModification
import pf.Setup.SameVoxelType
import pf.Comp.ReferenceFrame
import pf.Comp.ShadowableOmnidirectionalEmission
import pf.Comp.ShadowableUnidirectionalEmission
import pf.Comp.Motion
import pf.Setup.GeneratedVoxelObject
import pf.Setup.DynamicVoxels
import pf.Physics.AngularVelocity as AngularVelocity
import pf.Texture.TextureID
import pf.Setup.VoxelCollidable
import pf.Physics.ContactResponseParameters
import pf.Comp.DynamicGravity

import Entities.Player as Player

entity_ids = {
    ambient_light: Entity.id("ambient_light"),
    top_light: Entity.id("top_light"),
    bottom_light: Entity.id("bottom_light"),
    sun_light: Entity.id("sun_light"),
    sun: Entity.id("sun"),
    sphere: Entity.id("sphere"),
    voxelobj: Entity.id("voxelobj"),
    asteroid: Entity.id("asteroid"),
}

setup! : {} => Result {} Str
setup! = |_|
    Command.execute!(Engine(Scene(SetSkybox(Skybox.new(skybox, 2e3)))))?

    Player.setup!({})?

    Entity.create_with_id!(ambient_light, entity_ids.ambient_light)?
    Entity.create_with_id!(top_light, entity_ids.top_light)?
    Entity.create_with_id!(bottom_light, entity_ids.bottom_light)?
    Entity.create_with_id!(sun_light, entity_ids.sun_light)?
    Entity.create_with_id!(sun, entity_ids.sun)?
    # Entity.create_with_id!(asteroid, entity_ids.asteroid)?

    Ok({})

handle_keyboard_event! = Player.handle_keyboard_event!
handle_mouse_button_event! = Player.handle_mouse_button_event!

skybox = Texture.TextureID.from_name("space_skybox")

ambient_light =
    Entity.new_component_data
    |> Comp.AmbientEmission.add_new(Vector3.same(1000))

top_light =
    Entity.new_component_data
    |> Comp.ShadowableUnidirectionalEmission.add_new(
        Vector3.same(20000),
        UnitVector3.from((0.0, -1.0, 0.0)),
        2.0,
    )

bottom_light =
    Entity.new_component_data
    |> Comp.ShadowableUnidirectionalEmission.add_new(
        Vector3.same(5000),
        UnitVector3.from((0.0, 1.0, 0.0)),
        2.0,
    )

sun_light =
    Entity.new_component_data
    |> Comp.ReferenceFrame.add_unoriented((0, 0, 0))
    |> Comp.ShadowableOmnidirectionalEmission.add_new(
        Vector3.same(2e7),
        0.7,
    )

sun =
    Entity.new_component_data
    |> Comp.ReferenceFrame.add_unoriented((0, 0, 0))
    |> Setup.VoxelSphere.add_new(0.5, 200)
    |> Setup.MultifractalNoiseSDFModification.add_new(8, 0.02, 2.0, 0.6, 2.0, 0)
    |> Setup.SameVoxelType.add_new("Default")
    |> Comp.Motion.add_angular(AngularVelocity.new(UnitVector3.y_axis, Radians.from_degrees(0)))
    |> Setup.DynamicVoxels.add
    |> Setup.VoxelCollidable.add_new(
        Dynamic,
        Physics.ContactResponseParameters.new(0.01, 0.7, 0.5),
    )
    |> Comp.DynamicGravity.add

asteroid =
    Entity.new_component_data
    |> Comp.ReferenceFrame.add_unoriented((0, 0, 30))
    |> Setup.GeneratedVoxelObject.add_new("asteroid", 0.25, 0)
    |> Setup.SameVoxelType.add_new("Default")
    |> Comp.Motion.add_angular(AngularVelocity.new(UnitVector3.y_axis, Radians.from_degrees(0)))
    |> Setup.DynamicVoxels.add
    |> Setup.VoxelCollidable.add_new(
        Dynamic,
        Physics.ContactResponseParameters.new(0.2, 0.7, 0.5),
    )
