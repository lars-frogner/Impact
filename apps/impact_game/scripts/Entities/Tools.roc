module [
    ToolEntities,
    thruster,
    laser,
    absorber,
    projectile,
    spawn!,
    get_absorbed_mass!,
    spawn_projectile!,
]

import core.UnitQuaternion
import core.Point3 exposing [Point3]
import core.Vector3 exposing [Vector3]
import core.UnitVector3 exposing [UnitVector3]
import core.Sphere

import pf.Entity

import pf.Setup.SceneParent
import pf.Setup.CylinderMesh
import pf.Setup.SphereMesh
import pf.Setup.UniformColor
import pf.Setup.UniformRoughness
import pf.Setup.UniformSpecularReflectance
import pf.Setup.UniformEmissiveLuminance
import pf.Comp.ModelTransform
import pf.Comp.Motion
import pf.Comp.ReferenceFrame
import pf.Setup.DynamicRigidBodySubstance
import pf.Setup.SphericalCollidable
import pf.Physics.ContactResponseParameters
import pf.Comp.DynamicGravity
import pf.Setup.VoxelAbsorbingCapsule
import pf.Comp.ShadowableOmnidirectionalEmission
import pf.Comp.OmnidirectionalEmission
import pf.Comp.SceneEntityFlags
import pf.Lookup.CapsuleAbsorbedVoxelMass

import Util

ToolEntities : {
    laser : Entity.ComponentData,
    absorber : Entity.ComponentData,
}

ToolEntityIds : {
    laser : Entity.Id,
    absorber : Entity.Id,
}

thruster = {
    force: 2e3,
}

laser = {
    visual_radius: 0.02,
    range: 500.0,
    color: (0.9, 0.05, 0.05),
    emissive_luminance: 1e6,
    right_shift: 0.15,
    down_shift: 0.3,
    absorb_radius: 0.5,
}

absorber = {
    visual_radius: 0.05,
    color: (0.9, 0.05, 0.05),
    emissive_luminance: 1e6,
    light_color: (1.0, 0.2, 0.2),
    luminous_intensity: 1e5,
    forward_shift: 2.0,
    absorb_radius: 1.0,
    stored_fraction: 2e-3,
}

projectile = {
    radius: 0.3,
    color: (1.0, 0.78, 0.043),
    specular_reflectance_percent: 50.0,
    roughness: 0.3,
    luminous_intensity: 1e4,
    speed: 10.0,
    mass: 15.0,
    restitution_coef: 0.4,
    static_friction_coef: 0.6,
    dynamic_friction_coef: 0.6,
    forward_shift: 2.0,
}

spawn! : ToolEntityIds, Entity.Id => Result {} Str
spawn! = |entity_ids, parent|
    ents = construct_entities(parent)

    Entity.create_with_id!(ents.laser, entity_ids.laser)?
    Entity.create_with_id!(ents.absorber, entity_ids.absorber)?

    Ok({})

get_absorbed_mass! : ToolEntityIds => Result F32 Str
get_absorbed_mass! = |entity_ids|
    absorbed_mass = Lookup.CapsuleAbsorbedVoxelMass.get!(entity_ids.absorber)?.mass
    Ok(absorbed_mass)

spawn_projectile! : Point3, Vector3, UnitVector3 => Result Vector3 Str
spawn_projectile! = |position, start_velocity, direction|
    launch_position = Vector3.add(position, Vector3.scale(direction, projectile.forward_shift))
    launch_velocity = Vector3.add(start_velocity, Vector3.scale(direction, projectile.speed))

    projectile_ent =
        Entity.new_component_data
        |> Setup.SphereMesh.add_new(64)
        |> Setup.UniformColor.add(projectile.color)
        |> Setup.UniformSpecularReflectance.add_in_range_of(
            Setup.UniformSpecularReflectance.stone,
            projectile.specular_reflectance_percent,
        )
        |> Setup.UniformRoughness.add(projectile.roughness)
        |> Setup.UniformEmissiveLuminance.add(
            Util.compute_sphere_emissive_luminance(projectile.luminous_intensity, projectile.radius),
        )
        |> Comp.OmnidirectionalEmission.add_new(
            Vector3.scale(projectile.color, projectile.luminous_intensity),
            2 * projectile.radius,
        )
        |> Comp.ModelTransform.add_with_scale(2 * projectile.radius)
        |> Comp.ReferenceFrame.add_unoriented(launch_position)
        |> Comp.Motion.add_linear(launch_velocity)
        |> Setup.DynamicRigidBodySubstance.add_new(
            Util.compute_sphere_mass_density(projectile.radius, projectile.mass),
        )
        |> Setup.SphericalCollidable.add_new(
            Dynamic,
            Sphere.new(Point3.origin, projectile.radius),
            Physics.ContactResponseParameters.new(
                projectile.restitution_coef,
                projectile.static_friction_coef,
                projectile.dynamic_friction_coef,
            ),
        )
        |> Comp.DynamicGravity.add

    Entity.stage_for_creation!(projectile_ent)?

    reaction_impulse = Vector3.scale(launch_velocity, -projectile.mass)

    Ok(reaction_impulse)

construct_entities : Entity.Id -> ToolEntities
construct_entities = |parent|
    laser_ent =
        Entity.new_component_data
        |> Setup.SceneParent.add_new(parent)
        |> Setup.CylinderMesh.add_new(laser.range, 2 * laser.visual_radius, 16)
        |> Setup.UniformColor.add(laser.color)
        |> Setup.UniformEmissiveLuminance.add(laser.emissive_luminance)
        |> Comp.ReferenceFrame.add_new(
            (laser.right_shift, -laser.down_shift, 0.0),
            UnitQuaternion.from_axis_angle(UnitVector3.unit_x, (-Num.pi) / 2),
        )
        |> Setup.VoxelAbsorbingCapsule.add_new(
            Vector3.same(0),
            (0, laser.range, 0),
            laser.absorb_radius,
        )
        |> Comp.SceneEntityFlags.add(
            Comp.SceneEntityFlags.union(
                Comp.SceneEntityFlags.is_disabled,
                Comp.SceneEntityFlags.casts_no_shadows,
            ),
        )

    absorber_ent =
        Entity.new_component_data
        |> Setup.SceneParent.add_new(parent)
        |> Setup.SphereMesh.add_new(64)
        |> Setup.UniformColor.add(absorber.color)
        |> Setup.UniformEmissiveLuminance.add(absorber.emissive_luminance)
        |> Comp.ShadowableOmnidirectionalEmission.add_new(
            Vector3.scale(absorber.light_color, absorber.luminous_intensity),
            2 * absorber.visual_radius,
        )
        |> Comp.ModelTransform.add_with_scale(2 * absorber.visual_radius)
        |> Comp.ReferenceFrame.add_unoriented((0, 0, -absorber.forward_shift))
        |> Setup.VoxelAbsorbingCapsule.add_new(
            (0, 0, 0),
            (0, 0, absorber.forward_shift - absorber.absorb_radius),
            absorber.absorb_radius,
        )
        |> Comp.SceneEntityFlags.add(Comp.SceneEntityFlags.is_disabled)

    { laser: laser_ent, absorber: absorber_ent }
