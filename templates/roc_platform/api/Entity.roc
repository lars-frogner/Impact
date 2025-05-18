module [
    Id,
    Data,
    MultiData,
    new_id,
    new,
    new_multi,
    append_component,
    append_components,
    create_with_id!,
    create!,
    create_multiple!,
    write_bytes_id,
    from_bytes_id,
]

import core.Builtin
import core.Hashing
import Platform

Id := U64 implements [Eq]

Data := List U8

MultiData := List U8

new_id : Str -> Id
new_id = |string|
    @Id(Hashing.hash_str_64(string) |> Hashing.unwrap_u64)

new = @Data([])
new_multi = @MultiData([])

append_component : Data, (List U8, a -> List U8), a -> Data
append_component = |@Data(bytes), encode, value|
    @Data(bytes |> encode(value))

append_components : MultiData, (List U8, List a -> List U8), List a -> MultiData
append_components = |@MultiData(bytes), encode, values|
    @MultiData(bytes |> encode(values))

create_with_id! : Id, Data => Result {} Str
create_with_id! = |@Id(id), @Data(bytes)|
    Platform.create_entity_with_id!(id, bytes)

create! : Data => Result Id Str
create! = |@Data(bytes)|
    Platform.create_entity!(bytes) |> Result.map_ok(@Id)

create_multiple! : MultiData => Result (List Id) Str
create_multiple! = |@MultiData(component_bytes)|
    Ok(Platform.create_entities!(component_bytes)? |> List.map(@Id))

write_bytes_id : List U8, Id -> List U8
write_bytes_id = |bytes, @Id(id)|
    Builtin.write_bytes_u64(bytes, id)

from_bytes_id : List U8 -> Result Id Builtin.DecodeErr
from_bytes_id = |bytes|
    Builtin.from_bytes_u64(bytes) |> Result.map_ok(@Id)
