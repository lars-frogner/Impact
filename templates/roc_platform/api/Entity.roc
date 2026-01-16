module [
    Id,
    ComponentData,
    MultiComponentData,
    ComponentIds,
    ReadComponentErr,
    id,
    new_component_data,
    new_multi_component_data,
    new_component_ids,
    multi_count,
    append_component,
    append_components,
    append_component_id,
    read_component,
    stage_for_creation_with_id!,
    stage_for_creation!,
    stage_multiple_for_creation!,
    stage_for_update!,
    stage_for_removal!,
    create_with_id!,
    create!,
    create_multiple!,
    update!,
    remove!,
    get_component!,
    get_components!,
    get_all_components!,
    write_bytes_id,
    from_bytes_id,
]

import core.Builtin
import core.Hashing
import Platform

Id := U64 implements [Eq]

ComponentData := List U8

MultiComponentData := { count : U64, bytes : List U8 }

ComponentIds := List U64

ReadComponentErr : [
    ComponentMissing,
    Decode Builtin.DecodeErr,
]

id : Str -> Id
id = |string|
    @Id(Hashing.hash_str_64(string) |> Hashing.to_u64)

new_component_data = @ComponentData([])

new_multi_component_data : U64 -> MultiComponentData
new_multi_component_data = |count|
    @MultiComponentData { count, bytes: [] }

new_component_ids = @ComponentIds([])

multi_count : MultiComponentData -> U64
multi_count = |@MultiComponentData { count }|
    count

append_component : ComponentData, (List U8, a -> List U8), a -> ComponentData
append_component = |@ComponentData(bytes), encode, value|
    @ComponentData(bytes |> encode(value))

append_components : MultiComponentData, (List U8, List a -> List U8), List a -> Result MultiComponentData [CountMismatch U64 U64]
append_components = |@MultiComponentData { count, bytes }, encode, values|
    value_count = List.len(values)
    if value_count == count then
        Ok(@MultiComponentData { count, bytes: bytes |> encode(values) })
    else
        Err(CountMismatch(value_count, count))

append_component_id : ComponentIds, U64 -> ComponentIds
append_component_id = |@ComponentIds(component_ids), component_id|
    @ComponentIds(component_ids |> List.append(component_id))

read_component : ComponentData, U64, (List U8 -> Result a Builtin.DecodeErr) -> Result a ReadComponentErr
read_component = |data, component_id, decode|
    bytes = find_component_bytes(data, component_id)?
    decode(bytes) |> Result.map_err(Decode)

find_component_bytes : ComponentData, U64 -> Result (List U8) ReadComponentErr
find_component_bytes = |@ComponentData(bytes), component_id|
    find_component_bytes_from_cursor(bytes, component_id, 0)

find_component_bytes_from_cursor : List U8, U64, U64 -> Result (List U8) ReadComponentErr
find_component_bytes_from_cursor = |bytes, target_component_id, cursor|
    if cursor + 24 <= List.len(bytes) then
        component_id =
            Builtin.from_bytes_u64(bytes |> List.sublist({ start: cursor, len: 8 }))
            |> Result.map_err(Decode)?
        component_size =
            Builtin.from_bytes_u64(bytes |> List.sublist({ start: cursor + 8, len: 8 }))
            |> Result.map_err(Decode)?

        # Skip alignment (8 bytes)

        if component_id == target_component_id then
            Ok(bytes |> List.sublist({ start: cursor + 24, len: component_size }))
        else
            find_component_bytes_from_cursor(bytes, target_component_id, cursor + 24 + component_size)
    else
        Err(ComponentMissing)

stage_for_creation_with_id! : ComponentData, Id => Result {} Str
stage_for_creation_with_id! = |@ComponentData(bytes), @Id(ident)|
    Platform.stage_entity_for_creation_with_id!(ident, bytes)

stage_for_creation! : ComponentData => Result {} Str
stage_for_creation! = |@ComponentData(bytes)|
    Platform.stage_entity_for_creation!(bytes)

stage_multiple_for_creation! : MultiComponentData => Result {} Str
stage_multiple_for_creation! = |@MultiComponentData { bytes }|
    Platform.stage_entities_for_creation!(bytes)

stage_for_update! : ComponentData, Id => Result {} Str
stage_for_update! = |@ComponentData(bytes), @Id(ident)|
    Platform.stage_entity_for_update!(ident, bytes)

stage_for_removal! : Id => Result {} Str
stage_for_removal! = |@Id(ident)|
    Platform.stage_entity_for_removal!(ident)

create_with_id! : ComponentData, Id => Result {} Str
create_with_id! = |@ComponentData(bytes), @Id(ident)|
    Platform.create_entity_with_id!(ident, bytes)

create! : ComponentData => Result Id Str
create! = |@ComponentData(bytes)|
    Platform.create_entity!(bytes) |> Result.map_ok(@Id)

create_multiple! : MultiComponentData => Result (List Id) Str
create_multiple! = |@MultiComponentData { bytes }|
    Ok(Platform.create_entities!(bytes)? |> List.map(@Id))

update! : ComponentData, Id => Result {} Str
update! = |@ComponentData(bytes), @Id(ident)|
    Platform.update_entity!(ident, bytes)

remove! : Id => Result {} Str
remove! = |@Id(ident)|
    Platform.remove_entity!(ident)

get_component! : Id, U64 => Result ComponentData Str
get_component! = |@Id(ident), component_id|
    Platform.read_entity_components!(ident, [component_id]) |> Result.map_ok(@ComponentData)

get_components! : Id, ComponentIds => Result ComponentData Str
get_components! = |@Id(ident), @ComponentIds(component_ids)|
    Platform.read_entity_components!(ident, component_ids) |> Result.map_ok(@ComponentData)

get_all_components! : Id => Result ComponentData Str
get_all_components! = |@Id(ident)|
    Platform.read_entity_components!(ident, []) |> Result.map_ok(@ComponentData)

write_bytes_id : List U8, Id -> List U8
write_bytes_id = |bytes, @Id(ident)|
    Builtin.write_bytes_u64(bytes, ident)

from_bytes_id : List U8 -> Result Id Builtin.DecodeErr
from_bytes_id = |bytes|
    Builtin.from_bytes_u64(bytes) |> Result.map_ok(@Id)
