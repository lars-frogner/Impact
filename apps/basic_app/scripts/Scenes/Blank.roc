module [
    entities,
    setup!,
]

import core.Plane
import core.Point3
import core.Radians
import core.UnitQuaternion
import core.UnitVector3
import core.Vector3
import pf.Command
import pf.Entity
import pf.Skybox
import pf.Comp.AmbientEmission
import pf.Comp.MotionControl
import pf.Comp.OrientationControl
import pf.Comp.PerspectiveCamera
import pf.Comp.PlaneCollidable
import pf.Comp.RectangleMesh
import pf.Comp.ReferenceFrame
import pf.Comp.ShadowableUnidirectionalEmission
import pf.Comp.UniformColor
import pf.Comp.UniformRoughness
import pf.Comp.UniformSpecularReflectance
import pf.Comp.Velocity
import pf.Rendering.TextureID

entities = {
    player: Entity.new_id("player"),
    ground: Entity.new_id("ground"),
    ambient_light: Entity.new_id("ambient_light"),
    unidirectional_light: Entity.new_id("unidirectional_light"),
}

setup! : {} => Result {} Str
setup! = |_|
    Command.execute!(Scene(SetSkybox(Skybox.new(skybox, 1e5))))?

    Entity.create_with_id!(entities.player, player)?
    Entity.create_with_id!(entities.ground, ground)?
    Entity.create_with_id!(entities.ambient_light, ambient_light)?
    Entity.create_with_id!(entities.unidirectional_light, unidirectional_light)?

    Ok({})

skybox = Rendering.TextureID.from_name("space_skybox")

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

ground =
    Entity.new
    |> Comp.RectangleMesh.add_unit_square
    |> Comp.ReferenceFrame.add_unoriented_scaled(Point3.origin, 1000)
    |> Comp.PlaneCollidable.add_new(Static, Plane.new(UnitVector3.y_axis, 0.0))
    |> Comp.UniformColor.add((0.9, 0.9, 0.9))
    |> Comp.UniformSpecularReflectance.add(0.01)
    |> Comp.UniformRoughness.add(0.5)

ambient_light =
    Entity.new
    |> Comp.AmbientEmission.add_new(Vector3.scale((1.0, 1.0, 1.0), 2000000.0))

unidirectional_light =
    Entity.new
    |> Comp.ShadowableUnidirectionalEmission.add_new(
        Vector3.scale((1.0, 1.0, 1.0), 200000.0),
        UnitVector3.from((0.0, -1.0, 0.0)),
        2.0,
    )
