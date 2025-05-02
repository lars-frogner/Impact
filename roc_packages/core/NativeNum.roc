module [
    Usize,
    to_usize,
    write_bytes_usize,
    from_bytes_usize,
]

import Builtin

Usize : U64

to_usize = Num.to_u64

write_bytes_usize = Builtin.write_bytes_u64
from_bytes_usize = Builtin.from_bytes_u64
