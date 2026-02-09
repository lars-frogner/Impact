module [
    entity_ids,
    star,
    spawn!,
]

import core.Vector3
import core.Point3
import core.Sphere

import pf.Entity
import pf.Comp.ParentEntity
import pf.Comp.CanBeParent
import pf.Setup.SphereMesh
import pf.Setup.UniformColor
import pf.Setup.UniformEmissiveLuminance
import pf.Comp.ModelTransform
import pf.Comp.ReferenceFrame
import pf.Comp.Motion
import pf.Setup.DynamicRigidBodySubstance
import pf.Physics.ContactResponseParameters
import pf.Setup.SphericalCollidable
import pf.Comp.DynamicGravity
import pf.Comp.ShadowableOmnidirectionalEmission

import Generation.SolarSystem as SolarSystem

StarEntities : {
    star : Entity.ComponentData,
    star_light : Entity.ComponentData,
}

entity_ids = {
    star: Entity.id("star"),
    star_light: Entity.id("star_light"),
}

star = {
    color: (1.0, 1.0, 1.0),
    restitution_coef: 0.0,
    static_friction_coef: 8.0,
    dynamic_friction_coef: 8.0,
}

spawn! : SolarSystem.Star => Result {} Str
spawn! = |star_props|
    ents = construct_entities(star_props)

    Entity.create_with_id!(ents.star, entity_ids.star)?
    Entity.create_with_id!(ents.star_light, entity_ids.star_light)?

    Ok({})

construct_entities : SolarSystem.Star -> StarEntities
construct_entities = |{ radius, mass_density, luminous_intensity, emissive_luminance }|
    star_ent =
        Entity.new_component_data
        |> Comp.CanBeParent.add
        |> Setup.SphereMesh.add_new(100)
        |> Setup.UniformColor.add(star.color)
        |> Setup.UniformEmissiveLuminance.add(emissive_luminance)
        |> Comp.ModelTransform.add_with_scale(2 * radius)
        |> Comp.ReferenceFrame.add_unoriented(Point3.origin)
        |> Comp.Motion.add_stationary
        |> Setup.DynamicRigidBodySubstance.add_new(mass_density)
        |> Setup.SphericalCollidable.add_new(
            Dynamic,
            Sphere.new(Point3.origin, radius),
            Physics.ContactResponseParameters.new(
                star.restitution_coef,
                star.static_friction_coef,
                star.dynamic_friction_coef,
            ),
        )
        |> Comp.DynamicGravity.add

    star_light_ent =
        Entity.new_component_data
        |> Comp.ParentEntity.add(entity_ids.star)
        |> Comp.ReferenceFrame.add_unoriented(Point3.origin)
        |> Comp.ShadowableOmnidirectionalEmission.add_new(
            Vector3.scale(star.color, luminous_intensity),
            2 * radius,
        )

    { star: star_ent, star_light: star_light_ent }
