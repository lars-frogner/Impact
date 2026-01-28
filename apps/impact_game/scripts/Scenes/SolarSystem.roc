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

import Generation.Orbit
import Generation.SolarSystem

import Entities.Player as Player
import Entities.Star as Star
import Entities.SphericalBodies as SphericalBodies
import Entities.FreeCamera as FreeCamera
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

    Command.execute!(Engine(Physics(SetGravitationalConstant(system.properties.grav_const))))?

    Entity.create_with_id!(ambient_light_ent, entity_ids.ambient_light)?

    Star.spawn!(system.star)?
    SphericalBodies.spawn!(system.bodies)?

    player_distance = 5e3
    player_speed = Generation.Orbit.compute_mean_orbital_speed(system.properties.grav_const, system.star.mass, player_distance)
    player_position = (0.0, 0.0, -player_distance)
    player_orientation = UnitQuaternion.identity
    player_velocity = (-player_speed, 0.0, 0.0)

    Player.spawn!(player_position, player_orientation, player_velocity)?

    FreeCamera.spawn!(player_position, player_orientation)?

    radius_to_cover = 1.1 * system.properties.radius
    OverviewCamera.spawn!(radius_to_cover)?

    when ctx.interaction_mode is
        Player ->
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
