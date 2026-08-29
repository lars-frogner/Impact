module [
    entity_ids,
    setup!,
]

import core.Plane
import core.Radians
import core.UnitQuaternion
import core.UnitVector3
import core.Vector3
import pf.Entity
import pf.Particle.RateOverTime
import pf.Particle.ColorOverLifetime
import pf.Particle.ScaleOverLifetime
import pf.Particle.RateOverTime
import pf.Particle.SpawnDistribution
import pf.Particle.ParticleMaterialID as ParticleMaterialID
import pf.Comp.AmbientEmission
import pf.Comp.VelocityControl
import pf.Comp.AngularVelocityControl
import pf.Setup.PerspectiveCamera
import pf.Setup.PlanarCollidable
import pf.Setup.RectangleMesh
import pf.Comp.ReferenceFrame
import pf.Comp.ModelTransform
import pf.Comp.ShadowableUnidirectionalEmission
import pf.Setup.UniformColor
import pf.Setup.UniformRoughness
import pf.Setup.UniformSpecularReflectance
import pf.Comp.RigidMotion
import pf.Physics.ContactResponseParameters
import pf.Physics.AngularVelocity as AngularVelocity
import pf.Setup.BasicParticleEffect
import pf.Setup.SphereMesh
import pf.Setup.UniformEmissiveLuminance
import pf.Comp.ShadowableOmnidirectionalEmission
import pf.Comp.ParentEntity

entity_ids = {
    player: Entity.id("player"),
    ground: Entity.id("ground"),
    parent: Entity.id("parent"),
    emitter1: Entity.id("emitter1"),
    ambient_light: Entity.id("ambient_light"),
    omnidirectional_light: Entity.id("omnidirectional_light"),
    unidirectional_light: Entity.id("unidirectional_light"),
}

setup! : {} => Result {} Str
setup! = |_|
    Entity.create_with_id!(player, entity_ids.player)?
    Entity.create_with_id!(ground, entity_ids.ground)?
    Entity.create_with_id!(parent, entity_ids.parent)?
    Entity.create_with_id!(emitter1, entity_ids.emitter1)?
    Entity.create_with_id!(ambient_light, entity_ids.ambient_light)?
    Entity.create_with_id!(omnidirectional_light, entity_ids.omnidirectional_light)?
    Entity.create_with_id!(unidirectional_light, entity_ids.unidirectional_light)?

    Ok({})

player =
    Entity.new_component_data
    |> Comp.ReferenceFrame.add_new(
        (0.0, 2.0, 0.0),
        UnitQuaternion.from_axis_angle(UnitVector3.unit_y, Num.pi),
    )
    |> Comp.RigidMotion.add_stationary
    |> Comp.VelocityControl.add
    |> Comp.AngularVelocityControl.add_all_directions
    |> Setup.PerspectiveCamera.add_new(Radians.from_degrees(70), 0.01, 1000)

ground =
    Entity.new_component_data
    |> Setup.RectangleMesh.add_unit_square
    |> Comp.ModelTransform.add_with_scale(1000)
    |> Comp.ReferenceFrame.add_unoriented((0, -1, 0))
    |> Comp.RigidMotion.add_stationary
    |> Setup.UniformColor.add((0.9, 0.9, 0.9))
    |> Setup.UniformSpecularReflectance.add(0.01)
    |> Setup.UniformRoughness.add(0.5)
    |> Setup.PlanarCollidable.add_new(
        Static,
        Plane.new(UnitVector3.unit_y, 0.0),
        Physics.ContactResponseParameters.new(0.0, 0.0, 0.0),
    )

parent =
    Entity.new_component_data
    |> Comp.ReferenceFrame.add_new(
        (0.0, 2.0, 6.0),
        UnitQuaternion.from_axis_angle(UnitVector3.unit_x, 0.0),
    )
    |> Comp.RigidMotion.add_new(
        (0.0, 0, 0),
        AngularVelocity.new(UnitVector3.unit_z, Radians.from_degrees(100.0)),
    )

emitter1 =
    Entity.new_component_data
    |> Comp.ParentEntity.add(entity_ids.parent)
    |> Comp.ReferenceFrame.add_new(
        (0.0, 0.5, 0.0),
        UnitQuaternion.from_axis_angle(UnitVector3.unit_x, (-Num.pi) / 2),
    )
    |> Setup.BasicParticleEffect.add(
        {
            material_id: ParticleMaterialID.from_name("Default"),
            log2_particle_count: 8,
            rate_over_time: Particle.RateOverTime.constant(Num.pow_int(2, 8), 0.0, 5.0),
            cos_polar_spawn_angle: Particle.SpawnDistribution.uniform_in_cos(Radians.from_degrees(0), Radians.from_degrees(30)),
            spawn_extent: Particle.SpawnDistribution.uniform(0.2, 0.4),
            spawn_distance: Particle.SpawnDistribution.uniform(0.0, 1.0),
            spawn_speed: Particle.SpawnDistribution.uniform(1.0, 2.0),
            spawn_angle: Particle.SpawnDistribution.uniform(0.0, 2 * Num.pi),
            spawn_angular_velocity: Particle.SpawnDistribution.uniform(-5.0, 5.0),
            lifetime: Particle.SpawnDistribution.uniform(5.0, 5.0),
            color_over_lifetime: Particle.ColorOverLifetime.linear_fade((1.0, 0.2, 0.5)),
            scale_over_lifetime: Particle.ScaleOverLifetime.linear_shrink({}),
        },
    )
# |> Comp.ReferenceFrame.add_new(
#    (0.0, 2.0, 2.0),
#    UnitQuaternion.from_axis_angle(UnitVector3.unit_x, -Num.pi),
# )
# |> Setup.BasicParticleEffect.add(
#    {
#        material_id: ParticleMaterialID.from_name("Default"),
#        log2_particle_count: 0,
#        rate_over_time: Particle.RateOverTime.burst(1, 0),
#        cos_polar_spawn_angle: Particle.SpawnDistribution.uniform_in_cos(Radians.from_degrees(0), Radians.from_degrees(90)),
#        spawn_extent: Particle.SpawnDistribution.uniform(1.0, 1.0),
#        spawn_distance: Particle.SpawnDistribution.uniform(0.0, 0.4),
#        spawn_speed: Particle.SpawnDistribution.uniform(0.0, 0.0),
#        spawn_angle: Particle.SpawnDistribution.same(0.0),
#        spawn_angular_velocity: Particle.SpawnDistribution.same(2.0),
#        lifetime: Particle.SpawnDistribution.same(10.0),
#        color_over_lifetime: Particle.ColorOverLifetime.linear_fade((1.0, 0.2, 0.5)),
#        scale_over_lifetime: Particle.ScaleOverLifetime.linear_shrink({}),
#    },
# )

ambient_light =
    Entity.new_component_data
    |> Comp.AmbientEmission.add_new(Vector3.same(20000))

omnidirectional_light =
    Entity.new_component_data
    |> Setup.SphereMesh.add_new(25)
    |> Comp.ModelTransform.add_with_scale(0.35)
    |> Comp.ReferenceFrame.add_unoriented((-5, 5, -5))
    |> Setup.UniformColor.add((1, 1, 1))
    |> Setup.UniformEmissiveLuminance.add(1e6)
    |> Comp.ShadowableOmnidirectionalEmission.add_new(
        Vector3.same(2e7),
        0.7,
    )

unidirectional_light =
    Entity.new_component_data
    |> Comp.ShadowableUnidirectionalEmission.add_new(
        Vector3.same(200000),
        UnitVector3.from((0.0, -1.0, 2.0)),
        2.0,
    )
