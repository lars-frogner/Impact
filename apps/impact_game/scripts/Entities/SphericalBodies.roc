module [
    body,
    spawn!,
]

import core.Point3
import core.Sphere
import core.ListUtil

import pf.Entity

import pf.Setup.SphereMesh
import pf.Setup.UniformColor
import pf.Setup.UniformSpecularReflectance
import pf.Setup.UniformRoughness
import pf.Comp.ModelTransform
import pf.Comp.ReferenceFrame
import pf.Comp.Motion
import pf.Setup.DynamicRigidBodySubstance
import pf.Physics.ContactResponseParameters
import pf.Setup.SphericalCollidable
import pf.Comp.DynamicGravity

import Generation.SolarSystem as SolarSystem

body = {
    color: (0.8, 0.8, 0.8),
    specular_reflectance_percent: 50.0,
    roughness: 0.6,
    mass_density: 1e3,
    restitution_coef: 0.0,
    static_friction_coef: 8.0,
    dynamic_friction_coef: 8.0,
}

spawn! : List SolarSystem.Body => Result {} Str
spawn! = |bodies|
    body_ents = construct_entities(bodies)?
    _ = Entity.create_multiple!(body_ents)?
    Ok({})

construct_entities : List SolarSystem.Body -> Result Entity.MultiComponentData Str
construct_entities = |bodies|

    (positions, velocities, diameters) =
        bodies
        |> List.map(|{ position, velocity, size }| (position, velocity, size))
        |> ListUtil.unzip3

    collidable_spheres =
        diameters
        |> List.map(|diameter| Sphere.new(Point3.origin, 0.5 * diameter))

    Entity.new_multi_component_data(List.len(bodies))
    |> Setup.SphereMesh.add_multiple_new(
        Same(100),
    )?
    |> Setup.UniformColor.add_multiple(
        Same(body.color),
    )?
    |> Setup.UniformSpecularReflectance.add_multiple_in_range_of(
        Same(Setup.UniformSpecularReflectance.stone),
        Same(body.specular_reflectance_percent),
    )?
    |> Setup.UniformRoughness.add_multiple(
        Same(body.roughness),
    )?
    |> Comp.ModelTransform.add_multiple_with_scale(
        All(diameters),
    )?
    |> Comp.ReferenceFrame.add_multiple_unoriented(
        All(positions),
    )?
    |> Comp.Motion.add_multiple_linear(
        All(velocities),
    )?
    |> Setup.DynamicRigidBodySubstance.add_multiple_new(
        Same(body.mass_density),
    )?
    |> Setup.SphericalCollidable.add_multiple_new(
        Same(Dynamic),
        All(collidable_spheres),
        Same(
            Physics.ContactResponseParameters.new(
                body.restitution_coef,
                body.static_friction_coef,
                body.dynamic_friction_coef,
            ),
        ),
    )?
    |> Comp.DynamicGravity.add_multiple
    |> Ok
