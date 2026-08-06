I'll start by reading the Rust source and the assembly region.

I read the source, the helper functions, and the full loop body in the assembly (`.LBB226_163`–`.LBB226_217`). Here's the analysis.

## Layout facts the codegen confirms

`Voxel` is 3 bytes `{type, signed_distance, flags}`, `CHUNK_SIZE = 16`, so strides are 3 / 48 / 768. All neighbor addresses are folded into constant displacements (`-766`, `770`, `-46`, `50`, `-1`, `5` off the row base) — `linear_voxel_idx_within_chunk` is fully constant-folded and **all bounds checks on chunk voxels are gone**. Addressing is as good as it gets.

## What the hot path actually costs

The common case — poly voxel empty _and_ original voxel unchanged (`.LBB226_171` → fallthrough → `.LBB226_170`) — is **~38 instructions / ~45 µops per voxel**, for work that amounts to two `max`es and three flag-clears. On top of that, per-row overhead (the two bbox blocks + face counts + the call) adds another **~6 µops/voxel amortized**. For a chunk entirely outside the polyhedron you're paying ~50 µops/voxel to do almost nothing.

---

## 1. `update_adjacencies*` — addressing is optimal, control flow is not

Two distinct problems:

**(a) Loop-invariant branches re-tested every voxel.** In `update_adjacencies_for_empty_voxel` (asm 2830–2877), four of the six guards — `i > 0`, `i + 1 < 16`, `j > 0`, `j + 1 < 16` — are invariant across the entire k-loop, yet they compile to `testq %r13,%r13 / je` and `cmpq $15,%r13 / je` **per voxel**. Same in the fallthrough path (2620–2642): three `test/je` pairs guarding three single-byte `andb`s. LLVM did not version the k-loop. That's ~6 branches per voxel guarding ~3 µops of real work.

**(b) `update_lower_adjacencies_for_non_empty_voxel` has three data-dependent, unpredictable branches.** Look at 2886–2952: each neighbor becomes a diamond (`cmpb $0,-766(%r15,%rax) / js .LBB226_201` → separate blocks `.LBB226_201/_202/_206`). Branching on a neighbor's sign bit is ~50/50 near the polyhedron surface — exactly where this path runs. These should be branchless:

```rust
let m = ((adjacent_voxel.signed_distance.encoded() >> 7) as u8); // 0xFF or 0x00
voxel.flags = (voxel.flags & !flag_for_current) | (flag_for_current & m);
adjacent_voxel.flags |= flag_for_adjacent & m;   // OR-with-0 when empty: same result
```

Making the adjacent write unconditional is semantically identical (OR with 0) and removes the branch entirely. That's the cheapest high-value fix in the whole loop.

For (a), the clean fix is not loop-peeling (9 versions of the body). Adjacency flags are a **pure function of the emptiness bitmask** — see §4.

## 2. `lower_occupied_k <= upper_occupied_k` — inefficient, and removable entirely

Verdict: **no, these are not efficient**, and worse, they're computing redundant information.

`.LBB226_214` and `.LBB226_215` each do 6 loads + 6 `cmp` + 6 `cmov` + **12 stores** — up to **~60 µops per row**, ~3.75 µops/voxel just for bbox tracking. No SIMD (answering your TODO at line 1017); LLVM kept them as 64-bit scalar `cmov` chains.

Two separate things are wrong:

**Double stores.** Every update writes the same value to two stack slots (`movq %rcx, 1096(%rsp)` + `movq %rcx, 928(%rsp)`, and so on throughout 3010–3097, 2501–2504, 2983–2986). LLVM is maintaining a shadow copy of `lower/upper_occupied_voxels` and `FaceCounts` because they're also read after the loop via `.iter().zip()` (line 1061) and `array::from_fn` (1089). Half the stores are pure waste.

**The i/j components don't need per-row granularity.** `i` is monotonically increasing, so `lower_occupied_voxels[0].min(i)` only ever fires once. `j` resets per i-plane. Both can be lifted out:

```rust
// inside the k-loop, replace lower_occupied_k/upper_occupied_k/row_occupied_count with:
row_mask |= ((sd as u8 as u16) >> 7) << k;

// after the k-loop — no branch:
let lower_k = row_mask.trailing_zeros() as usize;   // 16 if empty
let upper_k = 15 - row_mask.leading_zeros() as usize;
row_occupied_count = row_mask.count_ones() as usize;
j_occupied_mask |= ((row_mask != 0) as u16) << j;
lower_occupied_voxels[2] = lower_occupied_voxels[2].min(lower_k);
upper_occupied_voxels[2] = upper_occupied_voxels[2].max(upper_k);

// after the j-loop, once per i-plane (16× per chunk instead of 256×):
if j_occupied_mask != 0 { /* update i and j bounds */ }
```

This deletes both `if lower_occupied_k <= upper_occupied_k` blocks, drops 6 loop-carried accumulators out of the innermost loop (which is what forced the spills at `(%rsp)`, `112`, `192`, `72(%rsp)`), and cuts the per-row bbox cost from ~60 µops to ~6.

## 3. The `k == 0` / `k == CHUNK_SIZE - 1` face counts are free — delete them

Answering your TODO at line 999: those two compares per occupied voxel (2667–2670, 2689–2695) are pure redundancy. `z_dn` is incremented iff the k=0 voxel is occupied ⟺ `lower_occupied_k == 0`, and `z_up` iff `upper_occupied_k == 15`. So after the k-loop:

```rust
face_occupied_counts.add_z_dn((lower_occupied_k == 0) as usize);
face_occupied_counts.add_z_up((upper_occupied_k == 15) as usize);
```

(needs new `add_z_dn`/`add_z_up` on `FaceCounts`). Correct in the empty case too: `lower = 16`, `upper = 0` → both false. This removes two branches per occupied voxel and two of the four spilled z-counters (`240`, `256`, `264`, `600(%rsp)`).

Similarly, the `on_lower_x_face` branches (TODO at 1043) are invariant over the whole j/k nest — they only need evaluating once per i-plane, not per row. Branchless masking there would be a wash (they predict well); hoisting is the real win.

## 4. The structural win: adjacency flags from bitmasks

For a row with occupancy mask `m`, `Z_DN` flags are `m << 1`, `Z_UP` are `m >> 1`. `X_DN`/`X_UP` come from rows `(i∓1, j)`, `Y_DN`/`Y_UP` from `(i, j∓1)`. Storing one `u16` per row (256 rows = 512 B, L1-resident) lets you compute _all_ adjacency flags for the whole chunk in a second pass of pure bit ops — replacing ~12 288 branchy byte-RMWs per chunk with a few thousand straight-line ops. Bit→byte expansion is ~3 instructions with `vpbroadcastw`/`vpand`/`vpcmpeqb`, or `_pdep_u64`.

This also subsumes §1: no `i>0`/`k+1<16` guards at all, they become mask edges.

## 5. Smaller items

- **`compute_max_plane_signed_dists_for_row` is not inlined** despite `#[inline]` (`callq` at 2528) — 256 calls/chunk, and the surrounding spill/reload churn at 2519–2556 is why so many accumulators live on the stack. `#[inline(always)]` is worth trying.
- **Per-voxel bounds check on `voxel_type_densities`** (2700, 2710 → panic at `.LBB226_421`). The chunk-voxel accesses are unchecked but this one isn't. Hoist the density lookup or use `get_unchecked`.
- **Inertia accumulators round-trip through memory every voxel** (2781–2829): 6 loads + 6 stores of `self.source`/`self.destination`, plus reloading the base pointers. They're loop-invariant addresses; LLVM couldn't keep them in XMM registers. Accumulating into locals and flushing per chunk would remove ~12 memory ops per boundary voxel.
- **Two redundant induction variables.** `%r15 ≡ %r10 - 1` and `416(%rsp) ≡ %r11` — LLVM maintains four row pointers where two suffice, contributing to the pressure that forced `%r14` to spill (2536/2955).
- **Scalar `complement()`** — `incb / movzbl / cmovol / negb` (2591–2600), ~5 µops per voxel, is `_mm_sub_epi8(zero, _mm_adds_epi8(v, one))`: 2 instructions for 16 lanes. The two `max`es are `_mm_max_epi8`, 1 instruction each for 16 lanes. And `_mm_movemask_epi8` gives you the entire row occupancy mask of §2 in a single instruction — _if_ the signed distances were contiguous.
- **3-byte AoS `Voxel` is the blocker for all of the above** (4 `movzbl` loads, 3 `movb` stores per voxel at 2566–2568/2580–2588). SoA per chunk — separate `[u8; 4096]` arrays for type / signed_distance / flags — makes every row 16 contiguous bytes, which unlocks §2 and §4 as single-instruction operations. This is the big one, and also the most invasive.

## Suggested order

1. Branchless neighbor test in `update_lower_adjacencies_for_non_empty_voxel` (§1b) — small, local, removes the only unpredictable branches.
2. Derive z-face counts from `lower/upper_occupied_k` (§3) — a few lines, removes 2 branches/voxel + 2 spilled counters.
3. Bitmask-based row occupancy, deleting both `lower_occupied_k <= upper_occupied_k` blocks (§2) — removes ~54 µops/row and most of the register pressure.
4. `#[inline(always)]` on the row-distance helper + hoist the density bounds check (§5) — cheap, measurable.
5. SoA layout, then SIMD row processing + bitmask adjacency (§4/§5) — the ~10× path, but a real refactor.

Estimates are static µop counts, not measurements — I'd verify with `perf stat` on branch-misses and µops-retired before and after step 1–3, since those are the ones whose payoff depends on how boundary-heavy your actual workload is.
