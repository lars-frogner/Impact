module [
    ToolEntities,
    thruster,
    laser,
    absorbing_sphere,
    spawn!,
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
import pf.Comp.VoxelAbsorbingCapsule
import pf.Comp.VoxelAbsorbingSphere
import pf.Comp.ShadowableOmnidirectionalEmission
import pf.Comp.SceneEntityFlags

ToolEntities : {
    laser : Entity.ComponentData,
    absorbing_sphere : Entity.ComponentData,
}

ToolEntityIds : {
    laser : Entity.Id,
    absorbing_sphere : Entity.Id,
}

thruster = {
    acceleration: 10.0,
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

absorbing_sphere = {
    visual_radius: 0.05,
    color: (0.9, 0.05, 0.05),
    emissive_luminance: 1e6,
    light_color: (1.0, 0.2, 0.2),
    luminous_intensity: 1e5,
    forward_shift: 3.0,
    absorb_radius: 1.5,
}

projectile = {
    radius: 0.5,
    color: (0.1, 0.1, 0.9),
    specular_reflectance_percent: 50.0,
    roughness: 0.6,
    speed: 10.0,
    mass_density: 1e3,
    restitution_coef: 0.4,
    static_friction_coef: 0.6,
    dynamic_friction_coef: 0.6,
    forward_shift: 2.0,
}

spawn! : ToolEntityIds, Entity.Id => Result {} Str
spawn! = |entity_ids, parent|
    ents = construct_entities(parent)

    Entity.create_with_id!(ents.laser, entity_ids.laser)?
    Entity.create_with_id!(ents.absorbing_sphere, entity_ids.absorbing_sphere)?

    Ok({})

spawn_projectile! : Point3, Vector3, UnitVector3 => Result {} Str
spawn_projectile! = |position, start_velocity, direction|
    projectile_ent =
        Entity.new_component_data
        |> Setup.SphereMesh.add_new(64)
        |> Setup.UniformColor.add(projectile.color)
        |> Setup.UniformSpecularReflectance.add_in_range_of(
            Setup.UniformSpecularReflectance.stone,
            projectile.specular_reflectance_percent,
        )
        |> Setup.UniformRoughness.add(projectile.roughness)
        |> Comp.ModelTransform.add_with_scale(2 * projectile.radius)
        |> Comp.ReferenceFrame.add_unoriented(
            Vector3.add(position, Vector3.scale(direction, projectile.forward_shift)),
        )
        |> Comp.Motion.add_linear(
            Vector3.add(start_velocity, Vector3.scale(direction, projectile.speed)),
        )
        |> Setup.DynamicRigidBodySubstance.add_new(projectile.mass_density)
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

    Entity.stage_for_creation!(projectile_ent)

construct_entities : Entity.Id -> ToolEntities
construct_entities = |parent|
    laser_ent =
        Entity.new_component_data
        |> Setup.SceneParent.add_new(parent)
        |> Comp.ReferenceFrame.add_new(
            (laser.right_shift, -laser.down_shift, 0.0),
            UnitQuaternion.from_axis_angle(UnitVector3.unit_x, (-Num.pi) / 2),
        )
        |> Setup.CylinderMesh.add_new(laser.range, 2 * laser.visual_radius, 16)
        |> Setup.UniformColor.add(laser.color)
        |> Setup.UniformEmissiveLuminance.add(laser.emissive_luminance)
        |> Comp.VoxelAbsorbingCapsule.add_new(
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

    absorbing_sphere_ent =
        Entity.new_component_data
        |> Setup.SceneParent.add_new(parent)
        |> Comp.ModelTransform.add_with_scale(2 * absorbing_sphere.visual_radius)
        |> Comp.ReferenceFrame.add_unoriented((0, 0, -absorbing_sphere.forward_shift))
        |> Setup.SphereMesh.add_new(64)
        |> Setup.UniformColor.add(absorbing_sphere.color)
        |> Setup.UniformEmissiveLuminance.add(absorbing_sphere.emissive_luminance)
        |> Comp.ShadowableOmnidirectionalEmission.add_new(
            Vector3.scale(absorbing_sphere.light_color, absorbing_sphere.luminous_intensity),
            2 * absorbing_sphere.visual_radius,
        )
        |> Comp.VoxelAbsorbingSphere.add_new(
            Vector3.same(0),
            absorbing_sphere.absorb_radius,
        )
        |> Comp.SceneEntityFlags.add(Comp.SceneEntityFlags.is_disabled)

    { laser: laser_ent, absorbing_sphere: absorbing_sphere_ent }
