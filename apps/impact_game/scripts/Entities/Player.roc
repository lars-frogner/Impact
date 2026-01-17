module [
    PlayerMode,
    entity_ids,
    player,
    camera,
    spawn!,
]

import core.Radians
import core.UnitQuaternion exposing [UnitQuaternion]
import core.UnitVector3
import core.Vector3
import core.Matrix3
import core.Point3 exposing [Point3]
import core.Sphere

import pf.Entity

import pf.Setup.LocalForce
import pf.Setup.CylinderMesh
import pf.Setup.SphereMesh
import pf.Setup.CapsuleMesh
import pf.Setup.DynamicRigidBodyInertialProperties
import pf.Comp.AngularVelocityControl
import pf.Comp.AngularVelocityControlParent
import pf.Control.AngularVelocityControlDirections
import pf.Control.AngularVelocityControlFlags
import pf.Setup.SceneParent
import pf.Setup.PerspectiveCamera
import pf.Comp.ReferenceFrame
import pf.Comp.ModelTransform
import pf.Setup.SceneGraphGroup
import pf.Setup.SphereMesh
import pf.Comp.ShadowableOmnidirectionalEmission
import pf.Setup.UniformColor
import pf.Setup.UniformEmissiveLuminance
import pf.Comp.Motion
import pf.Comp.VoxelAbsorbingCapsule
import pf.Comp.VoxelAbsorbingSphere
import pf.Comp.SceneEntityFlags
import pf.Physics.ContactResponseParameters
import pf.Setup.LocalForce
import pf.Setup.SphericalCollidable
import pf.Comp.DynamicGravity
import pf.Setup.GravityAlignmentTorque

PlayerMode : [Active, Overview]

PlayerEntities : {
    player : Entity.ComponentData,
    player_body : Entity.ComponentData,
    player_head : Entity.ComponentData,
}

entity_ids = {
    player: Entity.id("player"),
    player_body: Entity.id("player_body"),
    player_head: Entity.id("player_head"),
    laser: Entity.id("laser"),
    absorbing_sphere: Entity.id("absorbing_sphere"),
}

player = {
    body_color: (0.2, 0.8, 0.2),
    body_segment_length: 1.2,
    body_radius: 0.4,
    head_elevation: 0.0,
    head_forward_shift: 0.2,
    mass: 1.0,
    moment_of_inertia: 10.0,
    restitution_coef: 0.0,
    static_friction_coef: 5.0,
    dynamic_friction_coef: 5.0,
    alignment_settling_time: 2.0,
    alignment_precession_damping: 2.0,
}

camera = {
    field_of_view: 70,
    near_distance: 0.01,
    view_distance: 1e4,
}

spawn! : Point3, UnitQuaternion => Result {} Str
spawn! = |position, orientation|
    ents = construct_entities(position, orientation)

    Entity.create_with_id!(ents.player, entity_ids.player)?
    Entity.create_with_id!(ents.player_body, entity_ids.player_body)?
    Entity.create_with_id!(ents.player_head, entity_ids.player_head)?

    Ok({})

construct_entities : Point3, UnitQuaternion -> PlayerEntities
construct_entities = |position, orientation|
    player_ent =
        Entity.new_component_data
        |> Setup.SceneGraphGroup.add
        |> Comp.ReferenceFrame.add_new(
            position,
            orientation,
        )
        |> Comp.Motion.add_stationary
        |> Comp.AngularVelocityControl.add_new(
            Control.AngularVelocityControlDirections.horizontal,
            Control.AngularVelocityControlFlags.preserve_existing_for_horizontal,
        )
        |> Comp.AngularVelocityControlParent.add({ entity_id: entity_ids.player })
        |> Setup.DynamicRigidBodyInertialProperties.add_new(
            player.mass,
            Vector3.zeros,
            Matrix3.diagonal(
                (
                    player.moment_of_inertia,
                    player.moment_of_inertia,
                    player.moment_of_inertia,
                ),
            ),
        )
        |> Setup.SphericalCollidable.add_new(
            Dynamic,
            Sphere.new(Point3.origin, player.body_radius),
            Physics.ContactResponseParameters.new(
                player.restitution_coef,
                player.static_friction_coef,
                player.dynamic_friction_coef,
            ),
        )
        |> Setup.LocalForce.add_new(
            Vector3.zeros,
            Point3.origin,
        )
        |> Comp.DynamicGravity.add
        |> Setup.GravityAlignmentTorque.add_new(
            UnitVector3.neg_unit_y,
            player.alignment_settling_time,
            0.0,
            player.alignment_precession_damping,
        )

    player_body_ent =
        Entity.new_component_data
        |> Setup.SceneParent.add_new(entity_ids.player)
        |> Setup.CapsuleMesh.add_new(
            player.body_segment_length,
            player.body_radius,
            128,
        )
        |> Setup.UniformColor.add(player.body_color)
        |> Comp.ReferenceFrame.add_unoriented(Point3.origin)

    player_head_ent =
        Entity.new_component_data
        |> Setup.SceneParent.add_new(entity_ids.player)
        |> Setup.SceneGraphGroup.add
        |> Comp.ReferenceFrame.add_new(
            (
                0.0,
                0.5 * player.body_segment_length + player.body_radius + player.head_elevation,
                (1.0 + player.head_forward_shift) * player.body_radius,
            ),
            UnitQuaternion.from_axis_angle(UnitVector3.unit_y, Num.pi),
        )
        |> Comp.Motion.add_stationary
        |> Comp.AngularVelocityControl.add_new(
            Control.AngularVelocityControlDirections.vertical,
            Control.AngularVelocityControlFlags.empty,
        )
        |> Setup.PerspectiveCamera.add_new(
            Radians.from_degrees(camera.field_of_view),
            camera.near_distance,
            camera.view_distance,
        )

    {
        player: player_ent,
        player_body: player_body_ent,
        player_head: player_head_ent,
    }
