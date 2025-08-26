module [
    Id,
    Data,
    MultiData,
    id,
    new,
    new_multi,
    multi_count,
    append_component,
    append_components,
    stage_for_creation_with_id!,
    stage_for_creation!,
    stage_multiple_for_creation!,
    stage_for_removal!,
    create_with_id!,
    create!,
    create_multiple!,
    remove!,
    write_bytes_id,
    from_bytes_id,
]

import core.Builtin
import core.Hashing
import Platform

Id := U64 implements [Eq]

Data := List U8

MultiData := { count : U64, bytes : List U8 }

id : Str -> Id
id = |string|
    @Id(Hashing.hash_str_64(string) |> Hashing.unwrap_u64)

new = @Data([])

new_multi : U64 -> MultiData
new_multi = |count|
    @MultiData { count, bytes: [] }

multi_count : MultiData -> U64
multi_count = |@MultiData { count }|
    count

append_component : Data, (List U8, a -> List U8), a -> Data
append_component = |@Data(bytes), encode, value|
    @Data(bytes |> encode(value))

append_components : MultiData, (List U8, List a -> List U8), List a -> Result MultiData [CountMismatch U64 U64]
append_components = |@MultiData { count, bytes }, encode, values|
    value_count = List.len(values)
    if value_count == count then
        Ok(@MultiData { count, bytes: bytes |> encode(values) })
    else
        Err(CountMismatch(value_count, count))

stage_for_creation_with_id! : Id, Data => Result {} Str
stage_for_creation_with_id! = |@Id(ident), @Data(bytes)|
    Platform.stage_entity_for_creation_with_id!(ident, bytes)

stage_for_creation! : Data => Result {} Str
stage_for_creation! = |@Data(bytes)|
    Platform.stage_entity_for_creation!(bytes)

stage_multiple_for_creation! : MultiData => Result {} Str
stage_multiple_for_creation! = |@MultiData { bytes }|
    Platform.stage_entities_for_creation!(bytes)

stage_for_removal! : Id => Result {} Str
stage_for_removal! = |@Id(ident)|
    Platform.stage_entity_for_removal!(ident)

create_with_id! : Id, Data => Result {} Str
create_with_id! = |@Id(ident), @Data(bytes)|
    Platform.create_entity_with_id!(ident, bytes)

create! : Data => Result Id Str
create! = |@Data(bytes)|
    Platform.create_entity!(bytes) |> Result.map_ok(@Id)

create_multiple! : MultiData => Result (List Id) Str
create_multiple! = |@MultiData { bytes }|
    Ok(Platform.create_entities!(bytes)? |> List.map(@Id))

remove! : Id => Result {} Str
remove! = |@Id(ident)|
    Platform.remove_entity!(ident)

write_bytes_id : List U8, Id -> List U8
write_bytes_id = |bytes, @Id(ident)|
    Builtin.write_bytes_u64(bytes, ident)

from_bytes_id : List U8 -> Result Id Builtin.DecodeErr
from_bytes_id = |bytes|
    Builtin.from_bytes_u64(bytes) |> Result.map_ok(@Id)
