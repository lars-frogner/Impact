module [
    Id,
    Data,
    MultiData,
    new,
    new_multi,
    append_component,
    append_components,
    create!,
    create_multiple!,
    enable!,
    disable!,
    write_bytes_id,
    from_bytes_id,
]

import Builtin
import Platform

Id := U64

Data := List U8

MultiData := List U8

new = @Data([])
new_multi = @MultiData([])

append_component : Data, (List U8, a -> List U8), a -> Data
append_component = |@Data(bytes), encode, value|
    @Data(bytes |> encode(value))

append_components : MultiData, (List U8, List a -> List U8), List a -> MultiData
append_components = |@MultiData(bytes), encode, values|
    @MultiData(bytes |> encode(values))

create! : Data => Result Id Str
create! = |@Data(bytes)|
    Platform.create_entity!(bytes) |> Result.map_ok(@Id)

create_multiple! : MultiData => Result (List Id) Str
create_multiple! = |@MultiData(component_bytes)|
    Ok(Platform.create_entities!(component_bytes)? |> List.map(@Id))

## Unsets the [`SceneEntityFlags::IS_DISABLED`] flag for the given entity.
##
## # Errors
## Returns an error if the entity does not exist or does not have the
## [`SceneEntityFlagsComp`] component.
enable! : Id => Result {} Str
enable! = |@Id(id)|
    Platform.enable_scene_entity!(id)

## Sets the [`SceneEntityFlags::IS_DISABLED`] flag for the given entity.
##
## # Errors
## Returns an error if the entity does not exist or does not have the
## [`SceneEntityFlagsComp`] component.
disable! : Id => Result {} Str
disable! = |@Id(id)|
    Platform.disable_scene_entity!(id)

write_bytes_id : List U8, Id -> List U8
write_bytes_id = |bytes, @Id(id)|
    Builtin.write_bytes_u64(bytes, id)

from_bytes_id : List U8 -> Result Id Builtin.DecodeErr
from_bytes_id = |bytes|
    Builtin.from_bytes_u64(bytes) |> Result.map_ok(@Id)
