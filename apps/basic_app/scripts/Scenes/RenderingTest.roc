module [
    entity_ids,
    setup!,
]

import core.Radians
import core.UnitQuaternion
import core.UnitVector3
import core.Vector3
import pf.Command
import pf.Entity
import pf.Skybox
import pf.Comp.AmbientEmission
import pf.Comp.BoxMesh
import pf.Comp.ConstantRotation
import pf.Comp.CylinderMesh
import pf.Comp.Mesh
import pf.Comp.MotionControl
import pf.Comp.NormalMap
import pf.Comp.OrientationControl
import pf.Comp.ParallaxMap
import pf.Comp.PerspectiveCamera
import pf.Comp.PlanarTextureProjection
import pf.Comp.RectangleMesh
import pf.Comp.ReferenceFrame
import pf.Comp.ShadowableOmnidirectionalEmission
import pf.Comp.ShadowableUnidirectionalEmission
import pf.Comp.SphereMesh
import pf.Comp.TexturedColor
import pf.Comp.TexturedRoughness
import pf.Comp.UniformColor
import pf.Comp.UniformEmissiveLuminance
import pf.Comp.UniformMetalness
import pf.Comp.UniformRoughness
import pf.Comp.UniformSpecularReflectance
import pf.Comp.Velocity
import pf.Mesh.MeshID as MeshID
import pf.Physics.AngularVelocity
import pf.Rendering.TextureID as TextureID

entity_ids = {
    player: Entity.id("player"),
    dragon: Entity.id("dragon"),
    pole: Entity.id("pole"),
    abstract_object: Entity.id("abstract_object"),
    abstract_pyramid: Entity.id("abstract_pyramid"),
    box: Entity.id("box"),
    sphere: Entity.id("sphere"),
    abstract_cube: Entity.id("abstract_cube"),
    floor: Entity.id("floor"),
    upper_x_wall: Entity.id("upper_x_wall"),
    lower_x_wall: Entity.id("lower_x_wall"),
    upper_z_wall: Entity.id("upper_z_wall"),
    bulb_light: Entity.id("bulb_light"),
    sun_light: Entity.id("sun_light"),
    ambient_light: Entity.id("ambient_light"),
}

setup! : {} => Result {} Str
setup! = |_|
    Command.execute!(Engine(Scene(SetSkybox(skybox))))?

    Entity.create_with_id!(entity_ids.player, player)?
    Entity.create_with_id!(entity_ids.dragon, dragon)?
    Entity.create_with_id!(entity_ids.pole, pole)?
    Entity.create_with_id!(entity_ids.abstract_object, abstract_object)?
    Entity.create_with_id!(entity_ids.abstract_pyramid, abstract_pyramid)?
    Entity.create_with_id!(entity_ids.box, box)?
    Entity.create_with_id!(entity_ids.sphere, sphere)?
    Entity.create_with_id!(entity_ids.abstract_cube, abstract_cube)?
    Entity.create_with_id!(entity_ids.floor, floor)?
    Entity.create_with_id!(entity_ids.upper_x_wall, upper_x_wall)?
    Entity.create_with_id!(entity_ids.lower_x_wall, lower_x_wall)?
    Entity.create_with_id!(entity_ids.upper_z_wall, upper_z_wall)?
    Entity.create_with_id!(entity_ids.bulb_light, bulb_light)?
    Entity.create_with_id!(entity_ids.sun_light, sun_light)?
    Entity.create_with_id!(entity_ids.ambient_light, ambient_light)?

    Ok({})

skybox = Skybox.new(TextureID.from_name("ocean_skybox"), 1e5)

player =
    Entity.new
    |> Comp.ReferenceFrame.add_unscaled(
        (0.0, 2.0, 0.0),
        UnitQuaternion.from_axis_angle(UnitVector3.y_axis, Num.pi),
    )
    |> Comp.Velocity.add_stationary
    |> Comp.MotionControl.add_new
    |> Comp.OrientationControl.add_new
    |> Comp.PerspectiveCamera.add_new(Radians.from_degrees(70), 0.01, 1000)

dragon =
    Entity.new
    |> Comp.Mesh.add_new(MeshID.from_name("dragon"))
    |> Comp.ReferenceFrame.add_new(
        (0.0, 3.5, 11.0),
        UnitQuaternion.from_axis_angle(UnitVector3.x_axis, (-Num.pi) / 2),
        0.06,
    )
    |> Comp.UniformColor.add((0.1, 0.2, 0.6))
    |> Comp.UniformSpecularReflectance.add_in_range_of(
        Comp.UniformSpecularReflectance.plastic,
        50,
    )
    |> Comp.UniformRoughness.add(0.4)

pole =
    Entity.new
    |> Comp.CylinderMesh.add_new(8.0, 0.6, 100)
    |> Comp.ReferenceFrame.add_unoriented((7.0, 0.0, 5.0))
    |> Comp.UniformColor.add_iron
    |> Comp.UniformSpecularReflectance.add_metal
    |> Comp.UniformMetalness.add_metal
    |> Comp.UniformRoughness.add(0.5)

abstract_object =
    Entity.new
    |> Comp.Mesh.add_new(MeshID.from_name("abstract_object"))
    |> Comp.ReferenceFrame.add_for_scaled_driven_rotation((7.0, 9.7, 5.0), 0.02)
    |> Comp.ConstantRotation.add_new(
        0,
        UnitQuaternion.from_axis_angle(UnitVector3.y_axis, 0),
        Physics.AngularVelocity.new(UnitVector3.y_axis, Radians.from_degrees(50)),
    )
    |> Comp.UniformColor.add_copper
    |> Comp.UniformSpecularReflectance.add_metal
    |> Comp.UniformMetalness.add_metal
    |> Comp.UniformRoughness.add(0.35)

abstract_pyramid =
    Entity.new
    |> Comp.Mesh.add_new(MeshID.from_name("abstract_pyramid"))
    |> Comp.ReferenceFrame.add_for_scaled_driven_rotation((-1.0, 11.0, 9.0), 0.035)
    |> Comp.ConstantRotation.add_new(
        0,
        UnitQuaternion.from_axis_angle(UnitVector3.x_axis, 0.4),
        Physics.AngularVelocity.new(UnitVector3.y_axis, Radians.from_degrees(-60)),
    )
    |> Comp.UniformColor.add((0.7, 0.3, 0.2))
    |> Comp.UniformRoughness.add(0.95)

box =
    Entity.new
    |> Comp.BoxMesh.add_unit_cube
    |> Comp.ReferenceFrame.add_unoriented_scaled((-9.0, 1.0, 5.0), 2.0)
    |> Comp.UniformColor.add((0.1, 0.7, 0.3))
    |> Comp.UniformSpecularReflectance.add_in_range_of(
        Comp.UniformSpecularReflectance.plastic,
        0.0,
    )
    |> Comp.UniformRoughness.add(0.55)

sphere =
    Entity.new
    |> Comp.SphereMesh.add_new(100)
    |> Comp.ReferenceFrame.add_unoriented_scaled((-9.0, 4.0, 5.0), 4.0)
    |> Comp.UniformColor.add((0.3, 0.2, 0.7))
    |> Comp.UniformSpecularReflectance.add_in_range_of(
        Comp.UniformSpecularReflectance.stone,
        50,
    )
    |> Comp.UniformRoughness.add(0.7)

abstract_cube =
    Entity.new
    |> Comp.Mesh.add_new(MeshID.from_name("abstract_cube"))
    |> Comp.ReferenceFrame.add_for_scaled_driven_rotation((-9.0, 7.8, 5.0), 0.016)
    |> Comp.ConstantRotation.add_new(
        0.0,
        UnitQuaternion.from_axis_angle(UnitVector3.y_axis, 0.7),
        Physics.AngularVelocity.new(UnitVector3.x_axis, Radians.from_degrees(30)),
    )
    |> Comp.UniformColor.add_gold
    |> Comp.UniformSpecularReflectance.add_metal
    |> Comp.UniformMetalness.add_metal
    |> Comp.UniformRoughness.add(0.4)

floor =
    Entity.new
    |> Comp.RectangleMesh.add_unit_square
    |> Comp.PlanarTextureProjection.add_for_rectangle(Comp.RectangleMesh.unit_square, 2, 2)
    |> Comp.ReferenceFrame.add_unoriented_scaled((0, 0, 0), 50)
    |> Comp.TexturedColor.add(TextureID.from_name("wood_floor_color_texture"))
    |> Comp.UniformSpecularReflectance.add_in_range_of(
        Comp.UniformSpecularReflectance.living_tissue,
        100.0,
    )
    |> Comp.TexturedRoughness.add_unscaled(TextureID.from_name("wood_floor_roughness_texture"))
    |> Comp.NormalMap.add(TextureID.from_name("wood_floor_normal_texture"))

wall_base =
    Entity.new
    |> Comp.RectangleMesh.add_unit_square
    |> Comp.PlanarTextureProjection.add_for_rectangle(Comp.RectangleMesh.unit_square, 2, 2)
    |> Comp.TexturedColor.add(TextureID.from_name("bricks_color_texture"))
    |> Comp.UniformSpecularReflectance.add(0.02)
    |> Comp.TexturedRoughness.add_unscaled(TextureID.from_name("bricks_roughness_texture"))
    |> Comp.ParallaxMap.add_new(TextureID.from_name("bricks_height_texture"), 0.02, (1 / 25, 1 / 25))

upper_x_wall =
    wall_base
    |> Comp.ReferenceFrame.add_new(
        (25, 5, 0),
        UnitQuaternion.from_axis_angle(UnitVector3.x_axis, Num.pi / 2)
        |> UnitQuaternion.mul(UnitQuaternion.from_axis_angle(UnitVector3.z_axis, Num.pi / 2)),
        50,
    )

lower_x_wall =
    wall_base
    |> Comp.ReferenceFrame.add_new(
        (-25, 5, 0),
        UnitQuaternion.from_axis_angle(UnitVector3.x_axis, Num.pi / 2)
        |> UnitQuaternion.mul(UnitQuaternion.from_axis_angle(UnitVector3.z_axis, (-Num.pi) / 2)),
        50,
    )

upper_z_wall =
    wall_base
    |> Comp.ReferenceFrame.add_new(
        (0, 5, 25),
        UnitQuaternion.from_axis_angle(UnitVector3.x_axis, (-Num.pi) / 2),
        50,
    )

bulb_light =
    Entity.new
    |> Comp.SphereMesh.add_new(25)
    |> Comp.ReferenceFrame.add_unoriented_scaled((0.0, 17.0, 2.0), 0.7)
    |> Comp.UniformColor.add((1.0, 1.0, 1.0))
    |> Comp.UniformEmissiveLuminance.add(1e6)
    |> Comp.ShadowableOmnidirectionalEmission.add_new(
        Vector3.same(2e7),
        0.7,
    )

sun_light =
    Entity.new
    |> Comp.ShadowableUnidirectionalEmission.add_new(
        Vector3.same(10000),
        UnitVector3.from((0.6, -0.3, 1.0)),
        2.0,
    )

ambient_light =
    Entity.new
    |> Comp.AmbientEmission.add_new(Vector3.same(1000.0))
