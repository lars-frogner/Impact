module [
    player,
    ground,
    ambient_light,
    unidirectional_light,
]

import core.Plane as Plane
import core.Point3 as Point3
import core.UnitQuaternion as UnitQuaternion
import core.UnitVector3 as UnitVector3
import core.Vector3 as Vector3
import pf.Entity as Entity
import Comp.AmbientEmission
import Comp.MotionControl
import Comp.OrientationControl
import Comp.PerspectiveCamera
import Comp.PlaneCollidable
import Comp.RectangleMesh
import Comp.ReferenceFrame
import Comp.ShadowableUnidirectionalEmission
import Comp.UniformColor
import Comp.UniformRoughness
import Comp.UniformSpecularReflectance
import Comp.Velocity

player = |_|
    Entity.new
    |> Comp.ReferenceFrame.add_unscaled(
        (0.0, 2.0, 0.0),
        UnitQuaternion.from_axis_angle(UnitVector3.y_axis, Num.pi),
    )
    |> Comp.Velocity.add_stationary
    |> Comp.MotionControl.add_new
    |> Comp.OrientationControl.add_new
    |> Comp.PerspectiveCamera.add_new(70, 0.01, 1000)

ground = |_|
    Entity.new
    |> Comp.RectangleMesh.add_unit_square
    |> Comp.ReferenceFrame.add_unoriented_scaled(Point3.origin, 1000)
    |> Comp.PlaneCollidable.add_new(Static, Plane.new(UnitVector3.y_axis, 0.0))
    |> Comp.UniformColor.add((0.9, 0.9, 0.9))
    |> Comp.UniformSpecularReflectance.add(0.01)
    |> Comp.UniformRoughness.add(0.5)

ambient_light = |_|
    Entity.new
    |> Comp.AmbientEmission.add_new(Vector3.scale((1.0, 1.0, 1.0), 2000000.0))

unidirectional_light = |_|
    Entity.new
    |> Comp.ShadowableUnidirectionalEmission.add_new(
        Vector3.scale((1.0, 1.0, 1.0), 200000.0),
        UnitVector3.from((0.0, -1.0, 0.0)),
        2.0,
    )
