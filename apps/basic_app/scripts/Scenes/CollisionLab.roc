module [
    entity_ids,
    setup!,
]

import core.ListUtil
import core.Plane
import core.Point3
import core.Radians
import core.Sphere
import core.UnitQuaternion
import core.UnitVector3 exposing [x_axis, y_axis, z_axis]
import core.Vector3
import pf.Command
import pf.Light.AmbientEmission
import pf.Setup.ConstantRotation
import pf.Comp.MotionControl
import pf.Setup.NormalMap
import pf.Light.OmnidirectionalEmission
import pf.Comp.OrientationControl
import pf.Setup.Parent
import pf.Setup.PerspectiveCamera
import pf.Setup.PlanarTextureProjection
import pf.Setup.PlanarCollidable
import pf.Setup.RectangleMesh
import pf.Comp.ReferenceFrame
import pf.Comp.ModelTransform
import pf.Comp.SameVoxelType
import pf.Setup.SceneGraphGroup
import pf.Light.ShadowableUnidirectionalEmission
import pf.Setup.SphericalCollidable
import pf.Setup.SphereMesh
import pf.Setup.TexturedColor
import pf.Setup.TexturedRoughness
import pf.Physics.ContactResponseParameters
import pf.Setup.ConstantAcceleration
import pf.Setup.DynamicRigidBodySubstance
import pf.Setup.UniformSpecularReflectance
import pf.Comp.Motion
import pf.Comp.VoxelBox
import pf.Setup.VoxelCollidable
import pf.Entity
import pf.Physics.AngularVelocity as AngularVelocity
import pf.Rendering.TextureID as TextureID
import pf.Skybox

entity_ids = {
    player: Entity.id("player"),
    sun_light: Entity.id("sun_light"),
    ambient_light: Entity.id("ambient_light"),
    voxel_box: Entity.id("voxel_box"),
}

setup! : {} => Result {} Str
setup! = |_|
    Command.execute!(Engine(Scene(SetSkybox(skybox))))?

    Entity.create_with_id!(entity_ids.player, player)?
    Entity.create_with_id!(entity_ids.sun_light, sun_light)?
    Entity.create_with_id!(entity_ids.ambient_light, ambient_light)?

    sphere_radius = 0.5
    n_y = 4
    room_extent = 16.0
    n_spheres_y = 2 * n_y + 1

    create_spheres!(
        sphere_radius,
        (4, n_y, 4),
        (0, (n_spheres_y - 1) * sphere_radius, 0),
        create_texture_ids("plastic"),
    )?

    create_room!(
        room_extent,
        20,
        create_texture_ids("concrete"),
    )?

    voxel_extent = 0.25
    box_size = 6.0
    # Entity.create_with_id!(entity_ids.voxel_box, voxel_box(voxel_extent, box_size, room_extent))?

    Ok({})

skybox = Skybox.new(TextureID.from_name("space_skybox"), 1e5)

player =
    Entity.new
    |> Comp.ReferenceFrame.add_new(
        (0, 0, -16),
        UnitQuaternion.from_axis_angle(y_axis, Num.pi),
    )
    |> Comp.Motion.add_stationary
    |> Comp.MotionControl.add_new
    |> Comp.OrientationControl.add_new
    |> Setup.PerspectiveCamera.add_new(Radians.from_degrees(70), 0.01, 1000)

sun_light =
    Entity.new
    |> Light.ShadowableUnidirectionalEmission.add_new(
        Vector3.same(200000),
        UnitVector3.from((0, -1, 0)),
        2.0,
    )

ambient_light =
    Entity.new
    |> Light.AmbientEmission.add_new(Vector3.same(2000000))

create_texture_ids = |texture_name| {
    color: TextureID.from_name("${texture_name}_color_texture"),
    roughness: TextureID.from_name("${texture_name}_roughness_texture"),
    normal: TextureID.from_name("${texture_name}_normal_texture"),
}

create_spheres! = |radius, (nx, ny, nz), center, texture_ids|
    half_extent_x = 2 * radius * Num.to_frac(nx)
    half_extent_y = 2 * radius * Num.to_frac(ny)
    half_extent_z = 2 * radius * Num.to_frac(nz)

    xs = ListUtil.linspace(center.0 - half_extent_x, center.0 + half_extent_x, 2 * nx + 1)
    ys = ListUtil.linspace(center.1 - half_extent_y, center.1 + half_extent_y, 2 * ny + 1)
    zs = ListUtil.linspace(center.2 - half_extent_z, center.2 + half_extent_z, 2 * nz + 1)

    positions = ListUtil.cartprod3(xs, ys, zs)

    _ =
        Entity.new_multi(List.len(positions))
        |> Setup.SphereMesh.add_multiple_new(
            Same(100),
        )?
        |> Comp.ModelTransform.add_multiple_with_scale(
            Same(Num.to_f32(2 * radius)),
        )?
        |> Comp.ReferenceFrame.add_multiple_unoriented(
            All(positions),
        )?
        |> Comp.Motion.add_multiple_stationary
        |> Setup.DynamicRigidBodySubstance.add_multiple(
            Same({ mass_density: 1.0 }),
        )?
        |> Setup.SphericalCollidable.add_multiple_new(
            Same(Dynamic),
            Same(Sphere.new(Point3.origin, radius)),
            Same(Physics.ContactResponseParameters.new(0.7, 0.5, 0.3)),
        )?
        |> Setup.ConstantAcceleration.add_multiple_earth
        |> Setup.TexturedColor.add_multiple(
            Same(texture_ids.color),
        )?
        |> Setup.UniformSpecularReflectance.add_multiple_in_range_of(
            Same(Setup.UniformSpecularReflectance.plastic),
            Same(0),
        )?
        |> Setup.TexturedRoughness.add_multiple_unscaled(
            Same(texture_ids.roughness),
        )?
        |> Setup.NormalMap.add_multiple(
            Same(texture_ids.normal),
        )?
        |> Setup.PlanarTextureProjection.add_multiple_for_rectangle(
            Same(Setup.RectangleMesh.unit_square),
            Same(0.2),
            Same(0.2),
        )?
        |> Entity.create_multiple!?

    Ok({})

create_room! = |extent, angular_speed, texture_ids|
    offset = 0.5

    half_extent = extent / 2
    plane_y = (-offset) * extent

    angular_velocity =
        AngularVelocity.new(z_axis, Radians.from_degrees(angular_speed))

    wall_orientations =
        [
            (x_axis, 0),
            (x_axis, Num.pi),
            (z_axis, Num.pi / 2),
            (z_axis, (-Num.pi) / 2),
            (x_axis, Num.pi / 2),
            (x_axis, (-Num.pi) / 2),
        ]
        |> List.map(|(axis, angle)| UnitQuaternion.from_axis_angle(axis, angle))

    wall_ids =
        Entity.new_multi(List.len(wall_orientations))
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
            Same(Plane.new(y_axis, plane_y)),
            Same(Physics.ContactResponseParameters.new(0.2, 0.7, 0.5)),
        )?
        |> Setup.TexturedColor.add_multiple(
            Same(texture_ids.color),
        )?
        |> Setup.UniformSpecularReflectance.add_multiple(
            Same(0.01),
        )?
        |> Setup.TexturedRoughness.add_multiple_unscaled(
            Same(texture_ids.roughness),
        )?
        |> Setup.NormalMap.add_multiple(
            Same(texture_ids.normal),
        )?
        |> Setup.PlanarTextureProjection.add_multiple_for_rectangle(
            Same(Setup.RectangleMesh.unit_square),
            Same(2),
            Same(2),
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
        Entity.new_multi(List.len(wall_ids_for_lights))
        |> Setup.Parent.add_multiple_new(
            All(wall_ids_for_lights),
        )?
        |> Comp.ModelTransform.add_multiple_with_scale(
            Same(Num.to_f32(0.2 / extent)),
        )?
        |> Comp.ReferenceFrame.add_multiple_unoriented(
            All(light_positions),
        )?
        |> Light.OmnidirectionalEmission.add_multiple_new(
            Same(Vector3.same(5e7)),
            Same(0.7),
        )?
        |> Entity.create_multiple!?

    Ok({})

voxel_box = |voxel_extent, box_size, room_extent|
    Entity.new
    |> Comp.VoxelBox.add_new(voxel_extent, box_size, box_size, box_size)
    |> Comp.SameVoxelType.add_new(1)
    |> Comp.ReferenceFrame.add_unoriented(
        (
            0.0,
            0.5 * voxel_extent * box_size - 0.5 * room_extent,
            0.0,
        ),
    )
    # |> Comp.Motion.add_angular(AngularVelocity.new(UnitVector3.y_axis, Radians.from_degrees(50)))
    |> Setup.VoxelCollidable.add_new(
        Static,
        Physics.ContactResponseParameters.new(0.2, 0.7, 0.5),
    )
