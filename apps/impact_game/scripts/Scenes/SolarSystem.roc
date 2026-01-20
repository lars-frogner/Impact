module [
    entity_ids,
    skybox,
    ambient_light,
    setup!,
]

import core.Vector3
import core.UnitQuaternion

import pf.Command
import pf.Entity

import pf.Game.SetupContext exposing [SetupContext]

import pf.Skybox
import pf.Texture.TextureID
import pf.Comp.AmbientEmission

import Generation.SolarSystem

import Entities.Player as Player
import Entities.Tools as Tools
import Entities.Star as Star
import Entities.SphericalBodies as SphericalBodies
import Entities.OverviewCamera as OverviewCamera

entity_ids = {
    ambient_light: Entity.id("ambient_light"),
}

skybox = {
    texture: "space_skybox",
    max_luminance: 2e3,
}

ambient_light = {
    illuminance: 1e3,
}

setup! : SetupContext, Generation.SolarSystem.System => Result {} Str
setup! = |ctx, system|
    skybox_texture_id = Texture.TextureID.from_name(skybox.texture)
    Command.execute!(Engine(Scene(SetSkybox(Skybox.new(skybox_texture_id, skybox.max_luminance)))))?

    max_light_reach = 2.5 * Num.max(Player.camera.view_distance, system.properties.radius)
    Command.execute!(Engine(Scene(SetMaxOmnidirectionalLightReach(max_light_reach))))?

    Command.execute!(Engine(Physics(SetGravitationalConstant(system.properties.grav_const))))?

    Entity.create_with_id!(ambient_light_ent, entity_ids.ambient_light)?

    Star.spawn!(system.star)?
    SphericalBodies.spawn!(system.bodies)?

    radius_to_cover = 1.1 * system.properties.radius
    OverviewCamera.spawn!(radius_to_cover)?

    Player.spawn!(
        (0.0, 0.0, 5e3),
        UnitQuaternion.identity,
    )?

    Tools.spawn!({})?

    when ctx.player_mode is
        Active ->
            Command.execute!(UI(SetInteractivity(Disabled)))?
            Command.execute!(Engine(Scene(SetActiveCamera { entity_id: Player.entity_ids.player_head })))?

        Overview ->
            Command.execute!(UI(SetInteractivity(Enabled)))?
            Command.execute!(Engine(Scene(SetActiveCamera { entity_id: OverviewCamera.entity_ids.camera })))?

    Ok({})

ambient_light_ent =
    Entity.new_component_data
    |> Comp.AmbientEmission.add_new(Vector3.same(ambient_light.illuminance))
