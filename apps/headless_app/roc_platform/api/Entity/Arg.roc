module [
    Broadcasted,
    broadcast,
    broadcasted_map1,
    broadcasted_map2,
    broadcasted_map3,
    broadcasted_map4,
]

Broadcasted a : [Same a, All (List a)]

broadcast : Broadcasted a, U64 -> List a
broadcast = |arg, count|
    when arg is
        Same(value) -> value |> List.repeat(count)
        All(values) -> values

broadcasted_map1 : Broadcasted a, U64, (a -> out) -> List out
broadcasted_map1 = |a, count, constructor|
    List.map(
        broadcast(a, count),
        constructor,
    )

broadcasted_map2 : Broadcasted a1, Broadcasted a2, U64, (a1, a2 -> out) -> List out
broadcasted_map2 = |a1, a2, count, constructor|
    List.map2(
        broadcast(a1, count),
        broadcast(a2, count),
        constructor,
    )

broadcasted_map3 : Broadcasted a1, Broadcasted a2, Broadcasted a3, U64, (a1, a2, a3 -> out) -> List out
broadcasted_map3 = |a1, a2, a3, count, constructor|
    List.map3(
        broadcast(a1, count),
        broadcast(a2, count),
        broadcast(a3, count),
        constructor,
    )

broadcasted_map4 : Broadcasted a1, Broadcasted a2, Broadcasted a3, Broadcasted a4, U64, (a1, a2, a3, a4 -> out) -> List out
broadcasted_map4 = |a1, a2, a3, a4, count, constructor|
    List.map4(
        broadcast(a1, count),
        broadcast(a2, count),
        broadcast(a3, count),
        broadcast(a4, count),
        constructor,
    )
