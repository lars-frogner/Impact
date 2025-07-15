# TODO

## Architecture

- Put `voxel` and possibly `gizmo` modules in separate crates.

- Look for a nicer and more granular way of reporting desynchronization of render resources.

- Centralize assignment of binding locations?

## Graphics

- Support clicking on rendered entities to display info based on their components.

- Look into clustered shading.

## Physics

- Support disabling rigid bodies.

- Implement sphere-based voxel collision detection.

- Add more constraints.

- Implement Voronoi fracturing.

## Math

- Replace `nalgebra` with custom library powered by `glam`.

## Scheduling

- Support inverse dependencies (specify in a task declaration that it runs before another task).

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

- Investigate where arena allocation could be beneficial.

## ECS

- Consider reducing usage of `RwLock` in `impact_ecs`. Investigate scheduler as an alternative to locks.

## Roc

- Add API for reading component data from script.

- Implement hot reloading of script.

- Support state/context in Roc script.

## Misc

- Fix black square flashes due to bloom filter propagating NaNs.

- Fix tiny gaps between chunk meshes due to numerical imprecision.

- Replace synchronization primitives from `std` with those from `parking_lot`.

- Use `inventory` for gathering defined tasks.
