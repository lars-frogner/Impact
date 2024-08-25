# TODO

## Graphics

- Change to "global" bloom: https://learnopengl.com/Guest-Articles/2022/Phys.-Based-Bloom.

- Generalize tone mapping pass to dynamic range compression pass and include high-quality gamma correction.

- Experiment with smooth voxel mapping: https://bonsairobo.medium.com/smooth-voxel-mapping-a-technical-deep-dive-on-real-time-surface-nets-and-texturing-ef06d0f8ca14.

- Integrate `egui` for dev GUI.

## Physics

- Support voxel objects as rigid bodies.

- Implement sphere-based voxel collision detection.

- Implement collision resolution.

## Performance

- Use single bind group for G-buffer attachments.

- Avoid buffering previous transforms for shadow mapping.

- Investigate ways to improve shadow map update performance.

- Move occlusion to the alpha channel of the ambient reflected luminance attachment.

## Misc

- Remove obsolete multisampling support.

- Add render pass logging and timings.

- Add support for switching to viewing specific render attachments.
