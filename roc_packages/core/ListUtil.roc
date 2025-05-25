module [
    cartprod2,
    cartprod3,
    unzip2,
    unzip3,
    meshgrid2,
    meshgrid3,
    linspace,
]

cartprod2 : List a, List b -> List (a, b)
cartprod2 = |xs, ys|
    xs |> List.join_map(|x| ys |> List.map(|y| (x, y)))

cartprod3 : List a, List b, List c -> List (a, b, c)
cartprod3 = |xs, ys, zs|
    xs
    |> List.join_map(|x| ys |> List.join_map(|y| zs |> List.map(|z| (x, y, z))))

unzip2 : List (a, b) -> (List a, List b)
unzip2 = |values|
    len = List.len(values)
    values
    |> List.walk(
        (List.with_capacity(len), List.with_capacity(len)),
        |(xs, ys), (x, y)|
            (List.append(xs, x), List.append(ys, y)),
    )

unzip3 : List (a, b, c) -> (List a, List b, List c)
unzip3 = |values|
    len = List.len(values)
    values
    |> List.walk(
        (List.with_capacity(len), List.with_capacity(len), List.with_capacity(len)),
        |(xs, ys, zs), (x, y, z)|
            (List.append(xs, x), List.append(ys, y), List.append(zs, z)),
    )

meshgrid2 : List a, List b -> (List a, List b)
meshgrid2 = |xs, ys|
    cartprod2(xs, ys) |> unzip2

meshgrid3 : List a, List b, List c -> (List a, List b, List c)
meshgrid3 = |xs, ys, zs|
    cartprod3(xs, ys, zs) |> unzip3

linspace : Frac a, Frac a, U64 -> List (Frac a)
linspace = |start, end, len|
    when len is
        0 -> []
        1 -> [start]
        _ ->
            delta = (end - start) / Num.to_frac(len - 1)
            List.range({ start: At 0, end: Length(len) })
            |> List.map(|idx| start + delta * Num.to_frac(idx))
