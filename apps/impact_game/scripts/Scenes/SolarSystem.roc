module [
    entity_ids,
    skybox,
    ambient_light,
    top_light,
    bottom_light,
    setup!,
]

import core.Vector3
import core.UnitVector3
import core.UnitQuaternion

import pf.Command
import pf.Entity

import pf.Skybox
import pf.Texture.TextureID
import pf.Comp.AmbientEmission
import pf.Comp.ShadowableUnidirectionalEmission

import Generation.SolarSystem

import Entities.Player as Player
import Entities.Tools as Tools
import Entities.Star as Star
import Entities.SphericalBodies as SphericalBodies
import Entities.OverviewCamera as OverviewCamera

entity_ids = {
    ambient_light: Entity.id("ambient_light"),
    top_light: Entity.id("top_light"),
    bottom_light: Entity.id("bottom_light"),
}

skybox = {
    texture: "space_skybox",
    max_luminance: 2e3,
}

ambient_light = {
    illuminance: 1e3,
}

top_light = {
    color: (1.0, 1.0, 1.0),
    perpendicular_illuminance: 2e4,
    direction: UnitVector3.neg_unit_y,
    angular_extent: 2.0,
}

bottom_light = {
    color: (1.0, 1.0, 1.0),
    perpendicular_illuminance: 5e3,
    direction: UnitVector3.unit_y,
    angular_extent: 2.0,
}

setup! : Generation.SolarSystem.System, Player.PlayerMode => Result {} Str
setup! = |system, player_mode|
    skybox_texture_id = Texture.TextureID.from_name(skybox.texture)
    Command.execute!(Engine(Scene(SetSkybox(Skybox.new(skybox_texture_id, skybox.max_luminance)))))?

    Entity.create_with_id!(ambient_light_ent, entity_ids.ambient_light)?
    Entity.create_with_id!(top_light_ent, entity_ids.top_light)?
    Entity.create_with_id!(bottom_light_ent, entity_ids.bottom_light)?

    Star.spawn!(system.star)?
    SphericalBodies.spawn!(system.bodies)?

    when player_mode is
        Active ->
            Player.spawn!(
                (0.0, 0.0, 1e3),
                UnitQuaternion.identity,
            )?
            Tools.spawn!({})?

        Overview ->
            radius_to_cover = 1.1 * system.properties.radius
            OverviewCamera.spawn!(radius_to_cover)?

    Ok({})

ambient_light_ent =
    Entity.new_component_data
    |> Comp.AmbientEmission.add_new(Vector3.same(ambient_light.illuminance))

top_light_ent =
    Entity.new_component_data
    |> Comp.ShadowableUnidirectionalEmission.add_new(
        Vector3.scale(top_light.color, top_light.perpendicular_illuminance),
        top_light.direction,
        top_light.angular_extent,
    )

bottom_light_ent =
    Entity.new_component_data
    |> Comp.ShadowableUnidirectionalEmission.add_new(
        Vector3.scale(bottom_light.color, bottom_light.perpendicular_illuminance),
        bottom_light.direction,
        bottom_light.angular_extent,
    )
