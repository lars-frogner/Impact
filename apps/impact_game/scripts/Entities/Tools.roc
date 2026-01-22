module [
    entity_ids,
    thruster,
    laser,
    absorbing_sphere,
    spawn!,
]

import core.UnitQuaternion
import core.UnitVector3
import core.Vector3

import pf.Entity

import pf.Setup.CylinderMesh
import pf.Setup.SphereMesh
import pf.Setup.SceneParent
import pf.Comp.ReferenceFrame
import pf.Comp.ModelTransform
import pf.Setup.SphereMesh
import pf.Comp.ShadowableOmnidirectionalEmission
import pf.Setup.UniformColor
import pf.Setup.UniformEmissiveLuminance
import pf.Comp.VoxelAbsorbingCapsule
import pf.Comp.VoxelAbsorbingSphere
import pf.Comp.SceneEntityFlags

import Entities.Player as Player

ToolEntities : {
    laser : Entity.ComponentData,
    absorbing_sphere : Entity.ComponentData,
}

entity_ids = {
    laser: Entity.id("laser"),
    absorbing_sphere: Entity.id("absorbing_sphere"),
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
    absorb_radius: 1.0,
    absorb_rate: 2000.0,
}

absorbing_sphere = {
    visual_radius: 0.05,
    color: (0.9, 0.05, 0.05),
    emissive_luminance: 1e6,
    light_color: (1.0, 0.2, 0.2),
    luminous_intensity: 1e5,
    forward_shift: 3.0,
    absorb_radius: 1.0,
    absorb_rate: 30.0,
}

spawn! : {} => Result {} Str
spawn! = |_|
    ents = construct_entities({})

    Entity.create_with_id!(ents.laser, entity_ids.laser)?
    Entity.create_with_id!(ents.absorbing_sphere, entity_ids.absorbing_sphere)?

    Ok({})

construct_entities : {} -> ToolEntities
construct_entities = |_|
    laser_ent =
        Entity.new_component_data
        |> Setup.SceneParent.add_new(Player.entity_ids.player_head)
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
            laser.absorb_rate,
        )
        |> Comp.SceneEntityFlags.add(
            Comp.SceneEntityFlags.union(
                Comp.SceneEntityFlags.is_disabled,
                Comp.SceneEntityFlags.casts_no_shadows,
            ),
        )

    absorbing_sphere_ent =
        Entity.new_component_data
        |> Setup.SceneParent.add_new(Player.entity_ids.player_head)
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
            absorbing_sphere.absorb_rate,
        )
        |> Comp.SceneEntityFlags.add(Comp.SceneEntityFlags.is_disabled)

    { laser: laser_ent, absorbing_sphere: absorbing_sphere_ent }
