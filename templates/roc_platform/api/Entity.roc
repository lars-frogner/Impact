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
]

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
