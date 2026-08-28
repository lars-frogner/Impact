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
import pf.Comp.DistanceTriggeredRules
import pf.Comp.ReferenceFrame
import pf.Comp.RigidMotion
import pf.Physics.AngularVelocity as AngularVelocity
import pf.Setup.GeneratedVoxelObject
# import pf.Setup.SameVoxelType
import pf.Setup.GradientNoiseVoxelTypes
import pf.Setup.DynamicVoxels
import pf.Setup.VoxelCollidable
import pf.Physics.ContactResponseParameters
import pf.Comp.FracturingProperties
import pf.Comp.DynamicGravity
import pf.Comp.AmbientEmission
import pf.Comp.UnidirectionalEmission
import pf.Comp.ShadowableUnidirectionalEmission

import Entities.Player as Player
import Entities.FreeCamera as FreeCamera
import Entities.OverviewCamera as OverviewCamera

entity_ids = {
    ambient_light: Entity.id("ambient_light"),
    star_light: Entity.id("star_light"),
    asteroid: Entity.id("asteroid"),
    asteroid2: Entity.id("asteroid2"),
}

skybox = {
    texture: "space_skybox",
    max_luminance: 2e3,
}

ambient_light = {
    illuminance: 1e3,
}

star = {
    illuminance: 5e4,
    angular_extent: 2.0,
}

setup! : SetupContext => Result {} Str
setup! = |ctx|
    skybox_texture_id = Texture.TextureID.from_name(skybox.texture)
    Command.execute!(Engine(Scene(SetSkybox(Skybox.new(skybox_texture_id, skybox.max_luminance)))))?

    Command.execute!(Engine(Physics(SetGravitationalConstant(1e-4))))?

    Entity.create_with_id!(ambient_light_ent, entity_ids.ambient_light)?
    Entity.create_with_id!(star_light_ent, entity_ids.star_light)?
    _ = Entity.create!(fill_light_ent)?

    Entity.create_with_id!(asteroid_ent, entity_ids.asteroid)?
    Entity.create_with_id!(asteroid2_ent, entity_ids.asteroid2)?

    player_position = (0.0, 0.0, -80)
    player_orientation = UnitQuaternion.identity
    player_velocity = Vector3.zeros

    Player.spawn!(player_position, player_orientation, player_velocity)?

    FreeCamera.spawn!(player_position, player_orientation)?

    OverviewCamera.spawn!(3e2)?

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

star_light_ent =
    Entity.new_component_data
    |> Comp.ShadowableUnidirectionalEmission.add_new(
        Vector3.same(star.illuminance),
        UnitVector3.from((1.0, 0.0, 0.6)),
        star.angular_extent,
    )

fill_light_ent =
    Entity.new_component_data
    |> Comp.UnidirectionalEmission.add_new(
        Vector3.same(0.2 * star.illuminance),
        UnitVector3.from((-1.0, -0.2, 0.5)),
        star.angular_extent,
    )

asteroid_ent =
    Entity.new_component_data
    |> Comp.DistanceTriggeredRules.add_removal(Player.entity_ids.player, 2e3)
    |> Comp.ReferenceFrame.add_new((0, 0, 0), UnitQuaternion.from_axis_angle(UnitVector3.unit_y, -8))
    |> Setup.GeneratedVoxelObject.add_new("asteroid", 0.25, 1.0, 0)
    # |> Setup.SameVoxelType.add_new("Default")
    |> Setup.GradientNoiseVoxelTypes.add_new(["Rock"], 2e-2, 1, 0)
    |> Comp.RigidMotion.add_new((-3.5, -1, 0), AngularVelocity.new(UnitVector3.unit_y, Radians.from_degrees(-20)))
    |> Setup.DynamicVoxels.add
    |> Setup.VoxelCollidable.add_new(Dynamic, response_params)
    |> Comp.FracturingProperties.add(fracturing_props)
    |> Comp.DynamicGravity.add

asteroid2_ent =
    Entity.new_component_data
    |> Comp.DistanceTriggeredRules.add_removal(Player.entity_ids.player, 2e3)
    |> Comp.ReferenceFrame.add_unoriented((-100, 9, -40))
    |> Setup.GeneratedVoxelObject.add_new("asteroid", 0.25, 0.5, 2)
    # |> Setup.SameVoxelType.add_new("Default")
    |> Setup.GradientNoiseVoxelTypes.add_new(["Rock"], 2e-2, 1, 1)
    |> Comp.RigidMotion.add_new(
        (35.5, 3, 10),
        AngularVelocity.new(UnitVector3.unit_z, Radians.from_degrees(-35)),
    )
    |> Setup.DynamicVoxels.add
    |> Setup.VoxelCollidable.add_new(Dynamic, response_params)
    |> Comp.FracturingProperties.add(fracturing_props)
    |> Comp.DynamicGravity.add

response_params = Physics.ContactResponseParameters.new(0.1, 0.7, 0.5)

fracturing_props = {
    fracturing_force: 1e6,
    shattering_pressure: 1.5e8,
    fragment_scale: 0.2,
    min_fragment_extent: 0.1,
    max_fragment_extent: 0.8,
}
