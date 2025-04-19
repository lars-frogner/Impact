module [
    Hash32,
    Hash64,
    StringHash32,
    StringHash64,
    write_bytes_hash_32,
    write_bytes_hash_64,
    write_bytes_string_hash_32,
    write_bytes_string_hash_64,
    from_bytes_hash_32,
    from_bytes_hash_64,
    from_bytes_string_hash_32,
    from_bytes_string_hash_64,
]

import Builtin

Hash32 := U32
Hash64 := U64
StringHash32 := U32
StringHash64 := U64

write_bytes_hash_32 : List U8, Hash32 -> List U8
write_bytes_hash_32 = |bytes, @Hash32(hash)|
    Builtin.write_bytes_u32(bytes, hash)

write_bytes_hash_64 : List U8, Hash64 -> List U8
write_bytes_hash_64 = |bytes, @Hash64(hash)|
    Builtin.write_bytes_u64(bytes, hash)

write_bytes_string_hash_32 : List U8, StringHash32 -> List U8
write_bytes_string_hash_32 = |bytes, @StringHash32(string_hash)|
    Builtin.write_bytes_u32(bytes, string_hash)

write_bytes_string_hash_64 : List U8, StringHash64 -> List U8
write_bytes_string_hash_64 = |bytes, @StringHash64(string_hash)|
    Builtin.write_bytes_u64(bytes, string_hash)

from_bytes_hash_32 : List U8 -> Result Hash32 Builtin.DecodeErr
from_bytes_hash_32 = |bytes|
    Builtin.from_bytes_u32(bytes) |> Result.map_ok(@Hash32)

from_bytes_hash_64 : List U8 -> Result Hash64 Builtin.DecodeErr
from_bytes_hash_64 = |bytes|
    Builtin.from_bytes_u64(bytes) |> Result.map_ok(@Hash64)

from_bytes_string_hash_32 : List U8 -> Result StringHash32 Builtin.DecodeErr
from_bytes_string_hash_32 = |bytes|
    Builtin.from_bytes_u32(bytes) |> Result.map_ok(@StringHash32)

from_bytes_string_hash_64 : List U8 -> Result StringHash64 Builtin.DecodeErr
from_bytes_string_hash_64 = |bytes|
    Builtin.from_bytes_u64(bytes) |> Result.map_ok(@StringHash64)
