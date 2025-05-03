# TODO

## Graphics

- Generalize tone mapping pass to dynamic range compression pass and include high-quality gamma correction.

- Integrate `egui` for dev GUI.

## Physics

- Implement sphere-based voxel collision detection.

- Implement collision resolution.

- Implement Voronoi fracturing.

## Performance

- Use single bind group for G-buffer attachments.

- Investigate ways to improve shadow map update performance (check out multiview render passes).

- Consider using `slotmap` instead of custom types: https://github.com/orlp/slotmap

- Consider replacing all `HashMap`s with `FxHashMap`s from `rustc-hash`: https://github.com/rust-lang/rustc-hash

- Consider moving from `nalgebra` to `glam`: https://github.com/bitshifter/glam-rs

- Consider using `wgpu::TextureFormat::Rg11b10Float` for appropriate attachments.

- Consider reducing usage of `RwLock` in `impact_ecs`. Investigate scheduler as an alternative to locks.

- Define consistent locking order for all lock-protected resources under `Application` to avoid deadlocks in parallel `Tasks` (with optional run-time verification to identify violations).

- Handle rendering of single-chunk voxel objects separately in a more lightweight manner.

## ECS

- Be smarter about entity representation and management to increase max number of entities without exceeding 64 bits.

- Consider reducing usage of `RwLock` in `impact_ecs`. Investigate scheduler as an alternative to locks.

## Roc

- Add support for bitflags.

## Misc

- Fix tiny gaps between chunk meshes due to numerical imprecision.
