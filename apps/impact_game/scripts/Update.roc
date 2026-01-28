module [
    update_world!,
]

import core.Vector3
import core.UnitQuaternion

import pf.Game.UpdateContext exposing [UpdateContext]

import pf.Comp.ReferenceFrame
import pf.Comp.Motion
import pf.Physics.AngularVelocity as AngularVelocity

import Entities.Player as Player
import Entities.FreeCamera as FreeCamera
import Entities.Tools as Tools

update_world! : UpdateContext => Result {} Str
update_world! = |ctx|
    update_world_player_mode!(ctx)?
    Ok({})

update_world_player_mode! = |ctx|
    Tools.update!(Player.entity_ids.tools)

update_world_free_camera_mode! = |ctx|
    Tools.update!(FreeCamera.entity_ids.tools)
