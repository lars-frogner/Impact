# Particle system

Simulation compute shaders populate storage buffer with world space position, velocity++ and base RGBA color for each particle.

Emitters must have a power-of-two particle count. When created, a free section of the GPU particle buffer is assigned by start index by looking at which sections are occupied by other emitters. If none is found, the buffer is recreated with double size, copying over existing particles. Emitters also specify a simulation enum variant, mapping to a compute shader, with associated parameter values. They also specify a texture atlas index and a constant property index mapping to a set of values for specular reflectance, roughness, metalness and emissive luminance.

Each simulation compute shader is invoked once for each associated emitter with the appropriate particle range and simulation parameters.

Constant properties (specular reflectance, roughness, metalness and emissive luminance) are written to a fixed uniform buffer on startup. For the geometry pass, a push constant provides texture atlas index and constant property uniform index. One pass per emitter with appropriate push constant and particle instance range. Or improve with indirect buffer and draw call.

Vertex shader takes instance index for accessing particle in storage buffer. Vertex buffer holds model space position (centered at origin, front face towards+z) and texture coords for quad. Apply view transform to world space particle position to get camera space position, then offset with model space vertex position. This gives correct billboarding. Use texture atlas index to offset texture coords for atlas. Vertex output is projected position, texture coords and base RGBA color.

Fragment shader samples normal+alpha texture. Compare sampled times base alpha against threshold in screen-door filter matrix. Matrix indices are computed from moduloed pixel indices (derived from position) and jittered based on the frame count so TAA can smooth out the pixel pattern. If alpha does not exceed the threshold, the fragment is discarded. Otherwise output to g-buffer.

## GPU resources

- Constant property uniform buffer.
- RGBA (normal + alpha) atlas texture.
- Vertex buffer for single quad (position + texture coords).
- Particle storage buffer (position, RGBA color, simulation state like velocity).
- Per simulation variant: compute shader and uniform buffer with parameters.
