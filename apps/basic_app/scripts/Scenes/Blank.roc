module [
    entity_ids,
    setup!,
]

import core.Plane
import core.Radians
import core.UnitQuaternion
import core.UnitVector3
import core.Vector3
import pf.Command
import pf.Entity
import pf.Skybox
import pf.Light.AmbientEmission
import pf.Comp.MotionControl
import pf.Comp.OrientationControl
import pf.Setup.PerspectiveCamera
import pf.Setup.PlanarCollidable
import pf.Setup.RectangleMesh
import pf.Comp.ReferenceFrame
import pf.Comp.ModelTransform
import pf.Light.ShadowableUnidirectionalEmission
import pf.Setup.UniformColor
import pf.Setup.UniformRoughness
import pf.Setup.UniformSpecularReflectance
import pf.Comp.Motion
import pf.Rendering.TextureID
import pf.Physics.ContactResponseParameters

entity_ids = {
    player: Entity.id("player"),
    ground: Entity.id("ground"),
    ambient_light: Entity.id("ambient_light"),
    unidirectional_light: Entity.id("unidirectional_light"),
}

setup! : {} => Result {} Str
setup! = |_|
    Command.execute!(Engine(Scene(SetSkybox(Skybox.new(skybox, 1e5)))))?

    Entity.create_with_id!(entity_ids.player, player)?
    Entity.create_with_id!(entity_ids.ground, ground)?
    Entity.create_with_id!(entity_ids.ambient_light, ambient_light)?
    Entity.create_with_id!(entity_ids.unidirectional_light, unidirectional_light)?

    Ok({})

skybox = Rendering.TextureID.from_name("space_skybox")

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

ground =
    Entity.new
    |> Setup.RectangleMesh.add_unit_square
    |> Comp.ModelTransform.add_with_scale(1000)
    |> Setup.PlanarCollidable.add_new(
        Static,
        Plane.new(UnitVector3.y_axis, 0.0),
        Physics.ContactResponseParameters.new(0.0, 0.0, 0.0),
    )
    |> Setup.UniformColor.add((0.9, 0.9, 0.9))
    |> Setup.UniformSpecularReflectance.add(0.01)
    |> Setup.UniformRoughness.add(0.5)

ambient_light =
    Entity.new
    |> Light.AmbientEmission.add_new(Vector3.same(2000000))

unidirectional_light =
    Entity.new
    |> Light.ShadowableUnidirectionalEmission.add_new(
        Vector3.same(200000),
        UnitVector3.from((0.0, -1.0, 0.0)),
        2.0,
    )
