module [
    entity_ids,
    skybox,
    ambient_light,
    setup!,
]

import core.Radians
import core.Vector3
import core.UnitVector3
import core.UnitQuaternion

import pf.Command
import pf.Entity

import pf.Game.SetupContext exposing [SetupContext]

import pf.Skybox
import pf.Texture.TextureID
import pf.Comp.ReferenceFrame
import pf.Comp.Motion
import pf.Physics.AngularVelocity as AngularVelocity
import pf.Setup.GeneratedVoxelObject
import pf.Setup.SameVoxelType
import pf.Setup.DynamicVoxels
import pf.Setup.VoxelCollidable
import pf.Physics.ContactResponseParameters
import pf.Comp.DynamicGravity
import pf.Comp.AmbientEmission
import pf.Comp.ShadowableUnidirectionalEmission

import Entities.Player as Player
import Entities.FreeCamera as FreeCamera
import Entities.OverviewCamera as OverviewCamera

entity_ids = {
    ambient_light: Entity.id("ambient_light"),
    star_light: Entity.id("star_light"),
    asteroid: Entity.id("asteroid"),
}

skybox = {
    texture: "space_skybox",
    max_luminance: 2e3,
}

ambient_light = {
    illuminance: 3e2,
}

star = {
    illuminance: 1e4,
    angular_extent: 2.0,
}

setup! : SetupContext => Result {} Str
setup! = |ctx|
    skybox_texture_id = Texture.TextureID.from_name(skybox.texture)
    Command.execute!(Engine(Scene(SetSkybox(Skybox.new(skybox_texture_id, skybox.max_luminance)))))?

    Command.execute!(Engine(Physics(SetGravitationalConstant(1e-4))))?

    Entity.create_with_id!(ambient_light_ent, entity_ids.ambient_light)?
    Entity.create_with_id!(star_light_ent, entity_ids.star_light)?

    Entity.create_with_id!(asteroid_ent, entity_ids.asteroid)?

    player_position = (0.0, 0.0, -50)
    player_orientation = UnitQuaternion.identity
    player_velocity = Vector3.zeros

    Player.spawn!(player_position, player_orientation, player_velocity)?

    FreeCamera.spawn!(player_position, player_orientation)?

    OverviewCamera.spawn!(3e2)?

    when ctx.player_mode is
        Dynamic ->
            Command.execute!(Engine(Scene(SetActiveCamera { entity_id: Player.entity_ids.player_head })))?

        FreeCamera ->
            Command.execute!(Engine(Scene(SetActiveCamera { entity_id: FreeCamera.entity_ids.camera })))?

        OverviewCamera ->
            Command.execute!(UI(SetInteractivity(Enabled)))?
            Command.execute!(Engine(Scene(SetActiveCamera { entity_id: OverviewCamera.entity_ids.camera })))?

    Ok({})

ambient_light_ent =
    Entity.new_component_data
    |> Comp.AmbientEmission.add_new(Vector3.same(ambient_light.illuminance))

star_light_ent =
    Entity.new_component_data
    |> Comp.ShadowableUnidirectionalEmission.add_new(
        Vector3.same(star.illuminance),
        UnitVector3.from((1.0, 0.0, 0.0)),
        star.angular_extent,
    )

asteroid_ent =
    Entity.new_component_data
    |> Comp.ReferenceFrame.add_unoriented((0, 0, 0))
    |> Setup.GeneratedVoxelObject.add_new("asteroid", 0.25, 1.0, 0)
    |> Setup.SameVoxelType.add_new("Default")
    |> Comp.Motion.add_angular(AngularVelocity.new(UnitVector3.unit_y, Radians.from_degrees(0)))
    |> Setup.DynamicVoxels.add
    |> Setup.VoxelCollidable.add_new(
        Dynamic,
        Physics.ContactResponseParameters.new(0.1, 0.01, 0.01),
    )
    |> Comp.DynamicGravity.add
