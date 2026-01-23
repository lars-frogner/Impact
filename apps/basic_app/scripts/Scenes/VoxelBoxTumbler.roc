module [
    entity_ids,
    setup!,
    handle_mouse_button_event!,
]

import core.ListUtil
import core.Plane
import core.Point3
import core.Radians
import core.UnitQuaternion
import core.UnitVector3 exposing [unit_x, unit_y, unit_z]
import core.Vector3
import pf.Comp.AmbientEmission
import pf.Setup.ConstantRotation
import pf.Comp.VelocityControl
import pf.Comp.OmnidirectionalEmission
import pf.Comp.AngularVelocityControl
import pf.Setup.SceneParent
import pf.Setup.PerspectiveCamera
import pf.Setup.PlanarCollidable
import pf.Setup.CylinderMesh
import pf.Setup.RectangleMesh
import pf.Comp.ReferenceFrame
import pf.Comp.ModelTransform
import pf.Setup.SameVoxelType
import pf.Setup.SceneGraphGroup
import pf.Comp.ShadowableOmnidirectionalEmission
import pf.Comp.ShadowableUnidirectionalEmission
import pf.Setup.SphereMesh
import pf.Setup.UniformColor
import pf.Setup.UniformRoughness
import pf.Setup.UniformEmissiveLuminance
import pf.Physics.ContactResponseParameters
import pf.Setup.ConstantAcceleration
import pf.Setup.UniformSpecularReflectance
import pf.Comp.Motion
import pf.Comp.VoxelAbsorbingCapsule
import pf.Comp.VoxelAbsorbingSphere
import pf.Comp.SceneEntityFlags
import pf.Setup.VoxelBox
import pf.Setup.VoxelCollidable
import pf.Setup.DynamicVoxels
import pf.Entity
import pf.Physics.AngularVelocity as AngularVelocity
import pf.Input.MouseButtonEvent exposing [MouseButtonEvent]
import InputHandling.MouseButton as MouseButtonInput

entity_ids = {
    player: Entity.id("player"),
    camera: Entity.id("camera"),
    laser: Entity.id("laser"),
    absorbing_sphere: Entity.id("absorbing_sphere"),
    sun_light: Entity.id("sun_light"),
    ambient_light: Entity.id("ambient_light"),
}

setup! : {} => Result {} Str
setup! = |_|
    Entity.create_with_id!(player, entity_ids.player)?
    Entity.create_with_id!(camera, entity_ids.camera)?
    Entity.create_with_id!(laser, entity_ids.laser)?
    Entity.create_with_id!(absorbing_sphere, entity_ids.absorbing_sphere)?
    Entity.create_with_id!(sun_light, entity_ids.sun_light)?
    Entity.create_with_id!(ambient_light, entity_ids.ambient_light)?

    voxel_extent = 0.25
    box_size = 16.0
    n_y = 1
    room_extent = 20.0
    n_spheres_y = 2 * n_y + 1

    create_voxel_boxes!(
        voxel_extent,
        box_size,
        (0, n_y, 0),
        (4, (n_spheres_y - 1) * 0.5 * voxel_extent * box_size - 8, 0),
    )?

    create_room!(
        room_extent,
        12,
    )?

    Ok({})

handle_mouse_button_event! : MouseButtonEvent => Result {} Str
handle_mouse_button_event! = |{ button, state }|
    when button is
        Left ->
            MouseButtonInput.toggle_scene_entity_active_state!(
                entity_ids.laser,
                state,
            )

        Right ->
            MouseButtonInput.toggle_scene_entity_active_state!(
                entity_ids.absorbing_sphere,
                state,
            )

        _ -> Ok({})

player =
    Entity.new_component_data
    |> Comp.ReferenceFrame.add_unoriented((0, 0, 20))
    |> Comp.Motion.add_stationary
    |> Comp.VelocityControl.add
    |> Comp.AngularVelocityControl.add_all_directions
    |> Setup.SceneGraphGroup.add

camera =
    Entity.new_component_data
    |> Setup.SceneParent.add_new(entity_ids.player)
    |> Setup.PerspectiveCamera.add_new(Radians.from_degrees(70), 0.01, 1000)

laser =
    Entity.new_component_data
    |> Setup.SceneParent.add_new(entity_ids.player)
    |> Comp.ReferenceFrame.add_new(
        (0.15, -0.3, 0.0),
        UnitQuaternion.from_axis_angle(UnitVector3.unit_x, (-Num.pi) / 2),
    )
    |> Setup.CylinderMesh.add_new(100, 0.02, 16)
    |> Setup.UniformColor.add((0.9, 0.05, 0.05))
    |> Setup.UniformEmissiveLuminance.add(5e7)
    |> Comp.VoxelAbsorbingCapsule.add_new(Vector3.same(0), (0, 100, 0), 0.3)
    |> Comp.SceneEntityFlags.add(
        Comp.SceneEntityFlags.union(
            Comp.SceneEntityFlags.is_disabled,
            Comp.SceneEntityFlags.casts_no_shadows,
        ),
    )

absorbing_sphere =
    Entity.new_component_data
    |> Setup.SceneParent.add_new(entity_ids.player)
    |> Comp.ModelTransform.add_with_scale(0.1)
    |> Comp.ReferenceFrame.add_unoriented((0, 0, -3))
    |> Setup.SphereMesh.add_new(64)
    |> Setup.UniformColor.add((0.9, 0.05, 0.05))
    |> Setup.UniformEmissiveLuminance.add(5e7)
    |> Comp.ShadowableOmnidirectionalEmission.add_new(Vector3.scale((1.0, 0.2, 0.2), 5e6), 0.2)
    |> Comp.VoxelAbsorbingSphere.add_new(Vector3.same(0), 1)
    |> Comp.SceneEntityFlags.add(Comp.SceneEntityFlags.is_disabled)

sun_light =
    Entity.new_component_data
    |> Comp.ShadowableUnidirectionalEmission.add_new(
        Vector3.same(20000),
        UnitVector3.from((0, -1, 0)),
        2.0,
    )

ambient_light =
    Entity.new_component_data
    |> Comp.AmbientEmission.add_new(Vector3.same(20000))

create_voxel_boxes! = |voxel_extent, box_size, (nx, ny, nz), center|
    box_extent = voxel_extent * box_size
    half_extent_x = box_extent * Num.to_frac(nx)
    half_extent_y = box_extent * Num.to_frac(ny)
    half_extent_z = box_extent * Num.to_frac(nz)

    xs = ListUtil.linspace(center.0 - half_extent_x, center.0 + half_extent_x, 2 * nx + 1)
    ys = ListUtil.linspace(center.1 - half_extent_y, center.1 + half_extent_y, 2 * ny + 1)
    zs = ListUtil.linspace(center.2 - half_extent_z, center.2 + half_extent_z, 2 * nz + 1)

    positions = ListUtil.cartprod3(xs, ys, zs)

    _ =
        Entity.new_multi_component_data(List.len(positions))
        |> Setup.VoxelBox.add_multiple_new(
            Same(voxel_extent),
            Same(box_size),
            Same(box_size),
            Same(box_size),
        )?
        |> Setup.SameVoxelType.add_multiple_new(
            Same("Default"),
        )?
        |> Comp.ReferenceFrame.add_multiple_unoriented(
            All(positions),
        )?
        |> Comp.Motion.add_multiple_stationary
        |> Setup.VoxelCollidable.add_multiple_new(
            Same(Dynamic),
            Same(Physics.ContactResponseParameters.new(0.2, 0.7, 0.5)),
        )?
        |> Setup.DynamicVoxels.add_multiple
        |> Setup.ConstantAcceleration.add_multiple_earth
        |> Entity.create_multiple!?

    Ok({})

create_room! = |extent, angular_speed|
    offset = 0.5

    half_extent = extent / 2
    plane_y = (-offset) * extent

    angular_velocity =
        AngularVelocity.new(unit_z, Radians.from_degrees(angular_speed))

    wall_orientations =
        [
            (unit_x, 0),
            (unit_x, Num.pi),
            (unit_z, Num.pi / 2),
            (unit_z, (-Num.pi) / 2),
            (unit_x, Num.pi / 2),
            (unit_x, (-Num.pi) / 2),
        ]
        |> List.map(|(axis, angle)| UnitQuaternion.from_axis_angle(axis, angle))

    wall_ids =
        Entity.new_multi_component_data(List.len(wall_orientations))
        |> Setup.RectangleMesh.add_multiple_unit_square
        |> Comp.ModelTransform.add_multiple_with_offset_and_scale(
            Same((0, offset, 0)),
            Same(Num.to_f32(extent)),
        )?
        |> Comp.ReferenceFrame.add_multiple_new(
            Same(Point3.origin),
            All(wall_orientations),
        )?
        |> Comp.Motion.add_multiple_angular(
            Same(angular_velocity),
        )?
        |> Setup.ConstantRotation.add_multiple_new(
            Same(0),
            All(wall_orientations),
            Same(angular_velocity),
        )?
        |> Setup.PlanarCollidable.add_multiple_new(
            Same(Static),
            Same(Plane.new(unit_y, plane_y)),
            Same(Physics.ContactResponseParameters.new(0.2, 0.7, 0.5)),
        )?
        |> Setup.UniformColor.add_multiple(
            Same((0.2, 0.7, 0.2)),
        )?
        |> Setup.UniformSpecularReflectance.add_multiple(
            Same(0.01),
        )?
        |> Setup.UniformRoughness.add_multiple(
            Same(0.5),
        )?
        |> Setup.SceneGraphGroup.add_multiple
        |> Entity.create_multiple!?

    wall_ids_for_lights =
        wall_ids
        |> List.map(|wall_id| List.repeat(wall_id, 4))
        |> List.join

    light_positions =
        ListUtil.cartprod2(
            [
                (-half_extent) + 0.1,
                half_extent - 0.1,
            ],
            [
                (-half_extent) + 0.1,
                half_extent - 0.1,
            ],
        )
        |> List.map(|(x, z)| (x, plane_y + 0.1, z))
        |> List.map(|coords| List.repeat(coords, List.len(wall_orientations)))
        |> List.join

    _ =
        Entity.new_multi_component_data(List.len(wall_ids_for_lights))
        |> Setup.SceneParent.add_multiple_new(
            All(wall_ids_for_lights),
        )?
        |> Comp.ModelTransform.add_multiple_with_scale(
            Same(Num.to_f32(0.2 / extent)),
        )?
        |> Comp.ReferenceFrame.add_multiple_unoriented(
            All(light_positions),
        )?
        |> Comp.OmnidirectionalEmission.add_multiple_new(
            Same(Vector3.same(5e6)),
            Same(0.7),
        )?
        |> Entity.create_multiple!?

    Ok({})
