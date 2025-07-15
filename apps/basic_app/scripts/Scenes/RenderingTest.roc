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
import pf.Setup.BoxMesh
import pf.Setup.ConstantRotation
import pf.Setup.CylinderMesh
import pf.Comp.MotionControl
import pf.Setup.NormalMap
import pf.Comp.OrientationControl
import pf.Setup.ParallaxMap
import pf.Setup.PerspectiveCamera
import pf.Setup.PlanarTextureProjection
import pf.Setup.RectangleMesh
import pf.Comp.ReferenceFrame
import pf.Comp.ModelTransform
import pf.Comp.ShadowableOmnidirectionalEmission
import pf.Comp.ShadowableUnidirectionalEmission
import pf.Setup.SphereMesh
import pf.Setup.TexturedColor
import pf.Setup.TexturedRoughness
import pf.Setup.UniformColor
import pf.Setup.UniformEmissiveLuminance
import pf.Setup.UniformMetalness
import pf.Setup.UniformRoughness
import pf.Setup.UniformSpecularReflectance
import pf.Comp.Motion
import pf.Mesh.TriangleMeshID as TriangleMeshID
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
    |> Comp.ReferenceFrame.add_new(
        (0.0, 2.0, 0.0),
        UnitQuaternion.from_axis_angle(UnitVector3.y_axis, Num.pi),
    )
    |> Comp.Motion.add_stationary
    |> Comp.MotionControl.add_new
    |> Comp.OrientationControl.add_new
    |> Setup.PerspectiveCamera.add_new(Radians.from_degrees(70), 0.01, 1000)

dragon =
    Entity.new
    |> TriangleMeshID.add_from_name("dragon")
    |> Comp.ModelTransform.add_with_scale(0.06)
    |> Comp.ReferenceFrame.add_new(
        (0.0, 3.5, 11.0),
        UnitQuaternion.from_axis_angle(UnitVector3.x_axis, (-Num.pi) / 2),
    )
    |> Setup.UniformColor.add((0.1, 0.2, 0.6))
    |> Setup.UniformSpecularReflectance.add_in_range_of(
        Setup.UniformSpecularReflectance.plastic,
        50,
    )
    |> Setup.UniformRoughness.add(0.4)

pole =
    Entity.new
    |> Setup.CylinderMesh.add_new(8.0, 0.6, 100)
    |> Comp.ReferenceFrame.add_unoriented((7.0, 0.0, 5.0))
    |> Setup.UniformColor.add_iron
    |> Setup.UniformSpecularReflectance.add_metal
    |> Setup.UniformMetalness.add_metal
    |> Setup.UniformRoughness.add(0.5)

abstract_object =
    Entity.new
    |> TriangleMeshID.add_from_name("abstract_object")
    |> Comp.ModelTransform.add_with_scale(0.02)
    |> Comp.ReferenceFrame.add_unoriented((7.0, 9.7, 5.0))
    |> Comp.Motion.add_stationary
    |> Setup.ConstantRotation.add_new(
        0,
        UnitQuaternion.from_axis_angle(UnitVector3.y_axis, 0),
        Physics.AngularVelocity.new(UnitVector3.y_axis, Radians.from_degrees(50)),
    )
    |> Setup.UniformColor.add_copper
    |> Setup.UniformSpecularReflectance.add_metal
    |> Setup.UniformMetalness.add_metal
    |> Setup.UniformRoughness.add(0.35)

abstract_pyramid =
    Entity.new
    |> TriangleMeshID.add_from_name("abstract_pyramid")
    |> Comp.ModelTransform.add_with_scale(0.035)
    |> Comp.ReferenceFrame.add_unoriented((-1.0, 11.0, 9.0))
    |> Comp.Motion.add_stationary
    |> Setup.ConstantRotation.add_new(
        0,
        UnitQuaternion.from_axis_angle(UnitVector3.x_axis, 0.4),
        Physics.AngularVelocity.new(UnitVector3.y_axis, Radians.from_degrees(-60)),
    )
    |> Setup.UniformColor.add((0.7, 0.3, 0.2))
    |> Setup.UniformRoughness.add(0.95)

box =
    Entity.new
    |> Setup.BoxMesh.add_unit_cube
    |> Comp.ModelTransform.add_with_scale(2.0)
    |> Comp.ReferenceFrame.add_unoriented((-9.0, 1.0, 5.0))
    |> Setup.UniformColor.add((0.1, 0.7, 0.3))
    |> Setup.UniformSpecularReflectance.add_in_range_of(
        Setup.UniformSpecularReflectance.plastic,
        0.0,
    )
    |> Setup.UniformRoughness.add(0.55)

sphere =
    Entity.new
    |> Setup.SphereMesh.add_new(100)
    |> Comp.ModelTransform.add_with_scale(4.0)
    |> Comp.ReferenceFrame.add_unoriented((-9.0, 4.0, 5.0))
    |> Setup.UniformColor.add((0.3, 0.2, 0.7))
    |> Setup.UniformSpecularReflectance.add_in_range_of(
        Setup.UniformSpecularReflectance.stone,
        50,
    )
    |> Setup.UniformRoughness.add(0.7)

abstract_cube =
    Entity.new
    |> TriangleMeshID.add_from_name("abstract_cube")
    |> Comp.ModelTransform.add_with_scale(0.016)
    |> Comp.ReferenceFrame.add_unoriented((-9.0, 7.8, 5.0))
    |> Comp.Motion.add_stationary
    |> Setup.ConstantRotation.add_new(
        0.0,
        UnitQuaternion.from_axis_angle(UnitVector3.y_axis, 0.7),
        Physics.AngularVelocity.new(UnitVector3.x_axis, Radians.from_degrees(30)),
    )
    |> Setup.UniformColor.add_gold
    |> Setup.UniformSpecularReflectance.add_metal
    |> Setup.UniformMetalness.add_metal
    |> Setup.UniformRoughness.add(0.4)

floor =
    Entity.new
    |> Setup.RectangleMesh.add_unit_square
    |> Setup.PlanarTextureProjection.add_for_rectangle(Setup.RectangleMesh.unit_square, 2, 2)
    |> Comp.ModelTransform.add_with_scale(50)
    |> Comp.ReferenceFrame.add_unoriented((0, 0, 0))
    |> Setup.TexturedColor.add(TextureID.from_name("wood_floor_color_texture"))
    |> Setup.UniformSpecularReflectance.add_in_range_of(
        Setup.UniformSpecularReflectance.living_tissue,
        100.0,
    )
    |> Setup.TexturedRoughness.add_unscaled(TextureID.from_name("wood_floor_roughness_texture"))
    |> Setup.NormalMap.add(TextureID.from_name("wood_floor_normal_texture"))

wall_base =
    Entity.new
    |> Setup.RectangleMesh.add_unit_square
    |> Setup.PlanarTextureProjection.add_for_rectangle(Setup.RectangleMesh.unit_square, 2, 2)
    |> Setup.TexturedColor.add(TextureID.from_name("bricks_color_texture"))
    |> Setup.UniformSpecularReflectance.add(0.02)
    |> Setup.TexturedRoughness.add_unscaled(TextureID.from_name("bricks_roughness_texture"))
    |> Setup.ParallaxMap.add_new(TextureID.from_name("bricks_height_texture"), 0.02, (1 / 25, 1 / 25))

upper_x_wall =
    wall_base
    |> Comp.ModelTransform.add_with_scale(50)
    |> Comp.ReferenceFrame.add_new(
        (25, 5, 0),
        UnitQuaternion.from_axis_angle(UnitVector3.x_axis, Num.pi / 2)
        |> UnitQuaternion.mul(UnitQuaternion.from_axis_angle(UnitVector3.z_axis, Num.pi / 2)),
    )

lower_x_wall =
    wall_base
    |> Comp.ModelTransform.add_with_scale(50)
    |> Comp.ReferenceFrame.add_new(
        (-25, 5, 0),
        UnitQuaternion.from_axis_angle(UnitVector3.x_axis, Num.pi / 2)
        |> UnitQuaternion.mul(UnitQuaternion.from_axis_angle(UnitVector3.z_axis, (-Num.pi) / 2)),
    )

upper_z_wall =
    wall_base
    |> Comp.ModelTransform.add_with_scale(50)
    |> Comp.ReferenceFrame.add_new(
        (0, 5, 25),
        UnitQuaternion.from_axis_angle(UnitVector3.x_axis, (-Num.pi) / 2),
    )

bulb_light =
    Entity.new
    |> Setup.SphereMesh.add_new(25)
    |> Comp.ModelTransform.add_with_scale(0.7)
    |> Comp.ReferenceFrame.add_unoriented((0.0, 17.0, 2.0))
    |> Setup.UniformColor.add((1.0, 1.0, 1.0))
    |> Setup.UniformEmissiveLuminance.add(1e6)
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
