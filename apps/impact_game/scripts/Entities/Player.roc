module [
    entity_ids,
    player,
    camera,
    spawn!,
    handle_absorbed_voxels!,
    launch_projectile!,
]

import core.Radians
import core.UnitQuaternion exposing [UnitQuaternion]
import core.UnitVector3
import core.Vector3 exposing [Vector3]
import core.Matrix3
import core.Point3 exposing [Point3]
import core.Sphere

import pf.Entity

import pf.Command
import pf.Setup.LocalForce
import pf.Setup.CapsuleMesh
import pf.Setup.DynamicRigidBodyInertialProperties
import pf.Comp.AngularVelocityControl
import pf.Comp.AngularVelocityControlParent
import pf.Control.AngularVelocityControlDirections
import pf.Control.AngularVelocityControlFlags
import pf.Comp.ParentEntity
import pf.Setup.PerspectiveCamera
import pf.Comp.ReferenceFrame
import pf.Comp.CanBeParent
import pf.Setup.UniformColor
import pf.Comp.Motion
import pf.Physics.ContactResponseParameters
import pf.Setup.LocalForce
import pf.Setup.SphericalCollidable
import pf.Comp.DynamicGravity
import pf.Setup.FixedDirectionAlignmentTorque
import pf.Lookup.InventoryMass
import pf.Lookup.LauncherLaunchSpeed

import Entities.Tools as Tools

PlayerEntities : {
    player : Entity.ComponentData,
    player_body : Entity.ComponentData,
    player_head : Entity.ComponentData,
}

entity_ids = {
    player: Entity.id("player"),
    player_body: Entity.id("player_body"),
    player_head: Entity.id("player_head"),
    tools: {
        laser: Entity.id("player_laser"),
        laser_model: Entity.id("player_laser_model"),
        absorber: Entity.id("player_absorber"),
        absorber_model: Entity.id("player_absorber_model"),
    },
}

player = {
    body_color: (0.2, 0.8, 0.2),
    body_segment_length: 1.2,
    body_radius: 0.4,
    head_elevation: 0.0,
    head_forward_shift: 0.2,
    initial_mass: 100.0,
    initial_moment_of_inertia: 100.0,
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

spawn! : Point3, UnitQuaternion, Vector3 => Result {} Str
spawn! = |position, orientation, velocity|
    ents = construct_entities(position, orientation, velocity)

    Entity.create_with_id!(ents.player, entity_ids.player)?
    Entity.create_with_id!(ents.player_body, entity_ids.player_body)?
    Entity.create_with_id!(ents.player_head, entity_ids.player_head)?

    Tools.spawn!(entity_ids.tools, entity_ids.player_head)?

    Ok({})

handle_absorbed_voxels! = |_|
    absorbed_mass = Tools.get_absorbed_mass!(entity_ids.tools)?
    stored_mass = absorbed_mass * Tools.absorber.stored_fraction

    if stored_mass > 0.0 then
        add_to_body = AddMassRetainingMotion {
            entity_id: entity_ids.player,
            additional_mass: stored_mass,
        }
        [
            Engine(Physics(add_to_body)),
            Game(AddMassToInventory(stored_mass)),
        ]
        |> List.for_each_try!(Command.execute!)
    else
        Ok({})

launch_projectile! = |_|
    inventory_mass = Lookup.InventoryMass.get!({})?.mass

    if inventory_mass < Tools.projectile.mass then
        # Not enough mass in inventory for a projectile
        return Ok([])
    else
        {}

    (head_position, head_orientation) = get_head_pose!({})?

    player_motion = Comp.Motion.get_for_entity!(entity_ids.player)?

    launch_speed = Lookup.LauncherLaunchSpeed.get!({})?.speed

    reaction_impulse = Tools.spawn_projectile!(
        entity_ids.player,
        head_position,
        player_motion.linear_velocity,
        UnitQuaternion.rotate_vector(head_orientation, UnitVector3.neg_unit_z),
        launch_speed,
    )?

    apply_impulse = ApplyImpulse {
        entity_id: entity_ids.player,
        impulse: reaction_impulse,
        relative_position: head_position,
    }

    remove_body_mass = AddMassRetainingMotion {
        entity_id: entity_ids.player,
        additional_mass: -Tools.projectile.mass,
    }

    Ok(
        [
            Engine(Physics(apply_impulse)),
            Engine(Physics(remove_body_mass)),
            Game(AddMassToInventory(-Tools.projectile.mass)),
        ],
    )

get_head_pose! = |_|
    player_frame = Comp.ReferenceFrame.get_for_entity!(entity_ids.player)?
    player_head_frame = Comp.ReferenceFrame.get_for_entity!(entity_ids.player_head)?

    position = Vector3.add(player_frame.position, UnitQuaternion.rotate_vector(player_frame.orientation, player_head_frame.position))
    orientation = UnitQuaternion.mul(player_frame.orientation, player_head_frame.orientation)

    Ok((position, orientation))

construct_entities : Point3, UnitQuaternion, Vector3 -> PlayerEntities
construct_entities = |position, orientation, velocity|
    player_ent =
        Entity.new_component_data
        |> Comp.CanBeParent.add
        |> Comp.ReferenceFrame.add_new(
            position,
            orientation,
        )
        |> Comp.Motion.add_linear(velocity)
        |> Comp.AngularVelocityControl.add_new(
            Control.AngularVelocityControlDirections.horizontal,
            Control.AngularVelocityControlFlags.preserve_existing_for_horizontal,
        )
        |> Comp.AngularVelocityControlParent.add({ entity_id: entity_ids.player })
        |> Setup.DynamicRigidBodyInertialProperties.add_new(
            player.initial_mass,
            Vector3.zeros,
            Matrix3.diagonal(
                (
                    player.initial_moment_of_inertia,
                    player.initial_moment_of_inertia,
                    player.initial_moment_of_inertia,
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
        |> Setup.FixedDirectionAlignmentTorque.add_new(
            UnitVector3.neg_unit_y,
            UnitVector3.neg_unit_y,
            player.alignment_settling_time,
            0.0,
            player.alignment_precession_damping,
        )

    player_body_ent =
        Entity.new_component_data
        |> Comp.ParentEntity.add(entity_ids.player)
        |> Setup.CapsuleMesh.add_new(
            player.body_segment_length,
            player.body_radius,
            128,
        )
        |> Setup.UniformColor.add(player.body_color)
        |> Comp.ReferenceFrame.add_unoriented(Point3.origin)

    player_head_ent =
        Entity.new_component_data
        |> Comp.ParentEntity.add(entity_ids.player)
        |> Comp.CanBeParent.add
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
