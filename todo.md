# TODO

## Features

- Try moving post-processing shaders to templates.

  - Add import functionality to templating system if deemed useful.

- Change to "global" bloom: https://learnopengl.com/Guest-Articles/2022/Phys.-Based-Bloom.

- Generalize tone mapping pass to dynamic range compression pass and include high-quality gamma correction.

- Implement new multi-primitive render pipeline for voxels.

- Experiment with temporal anti-aliasing.

- Support voxel trees as rigid bodies.

- Implement collision detection and resolution.

## Fixes

- Fix black boxes produced by Gaussian blur shader (probably due to NaN).

- Investigate stuttering.
