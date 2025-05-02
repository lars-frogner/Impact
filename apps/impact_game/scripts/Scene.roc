module [create_player]

import core.UnitQuaternion as UnitQuaternion
import core.UnitVector3 as UnitVector3
import pf.Entity as Entity
import Generated.MotionControlComp as MotionControlComp
import Generated.OrientationControlComp as OrientationControlComp
import Generated.PerspectiveCameraComp as PerspectiveCameraComp
import Generated.ReferenceFrameComp as ReferenceFrameComp
import Generated.VelocityComp as VelocityComp

create_player : {} -> Entity.Data
create_player = |_|
    Entity.new
    |> ReferenceFrameComp.add_unscaled(
        (0.0, 2.0, 0.0),
        UnitQuaternion.from_axis_angle(UnitVector3.y_axis, Num.pi),
    )
    |> VelocityComp.add_stationary
    |> MotionControlComp.add_new
    |> OrientationControlComp.add_new
    |> PerspectiveCameraComp.add_new(70, 0.01, 1000)
