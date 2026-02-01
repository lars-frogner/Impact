module [
    update_world!,
]

import pf.Game.UpdateContext exposing [UpdateContext]

import Entities.Player as Player

update_world! : UpdateContext => Result {} Str
update_world! = |ctx|
    when ctx.interaction_mode is
        Player ->
            update_world_player_mode!({})

        FreeCamera | OverviewCamera ->
            Ok({})

update_world_player_mode! = |_|
    Player.handle_absorbed_voxels!({})?
    Ok({})
