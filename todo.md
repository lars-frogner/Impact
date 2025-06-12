# TODO

## Graphics

- Support clicking on rendered entities to display info based on their components.

## Physics

- Implement sphere-based voxel collision detection.

- Add more constraints.

- Implement Voronoi fracturing.

## Performance

- Use single bind group for G-buffer attachments.

- Investigate ways to improve shadow map update performance (check out multiview render passes).

- Consider using `slotmap` instead of custom types: https://github.com/orlp/slotmap

- Consider replacing all `HashMap`s with `FxHashMap`s from `rustc-hash`: https://github.com/rust-lang/rustc-hash

- Consider moving from `nalgebra` to `glam`: https://github.com/bitshifter/glam-rs

- Consider using `wgpu::TextureFormat::Rg11b10Float` for appropriate attachments.

- Define consistent locking order for all lock-protected resources under `Engine` to avoid deadlocks in parallel `Tasks` (with optional run-time verification to identify violations).

- Handle rendering of single-chunk voxel objects separately in a more lightweight manner.

- Consider using Welzl's algorithm to compute better bounding spheres.

## ECS

- Consider reducing usage of `RwLock` in `impact_ecs`. Investigate scheduler as an alternative to locks.

## Roc

- Implement hot reloading of script.

- Support state/context in Roc script.

## Misc

- Fix tiny gaps between chunk meshes due to numerical imprecision.
