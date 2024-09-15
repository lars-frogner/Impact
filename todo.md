# TODO

## Graphics

- Generalize tone mapping pass to dynamic range compression pass and include high-quality gamma correction.

- Experiment with smooth voxel mapping: https://bonsairobo.medium.com/smooth-voxel-mapping-a-technical-deep-dive-on-real-time-surface-nets-and-texturing-ef06d0f8ca14.

- Integrate `egui` for dev GUI.

## Physics

- Support voxel objects as rigid bodies.

- Implement sphere-based voxel collision detection.

- Implement collision resolution.

## Performance

- Cull chunks that are obscured from at least three directions based on frustum direction.

- Use single bind group for G-buffer attachments.

- Investigate ways to improve shadow map update performance.

- Consider using `slotmap` instead of custom types: https://github.com/orlp/slotmap

- Consider replacing all `HashMap`s with `FxHashMap`s from `rustc-hash`: https://github.com/rust-lang/rustc-hash

- Consider moving from `ndarray` to `glam`: https://github.com/bitshifter/glam-rs

- Consider using `wgpu::TextureFormat::Rg11b10Float` for appropriate attachments.

- Consider reducing usage of `RwLock` in `impact_ecs`.
