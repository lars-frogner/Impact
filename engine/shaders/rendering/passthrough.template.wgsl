struct VertexOutput {
    @builtin(position) projectedPosition: vec4f,
}

struct FragmentOutput {
    @location(0) color: vec4f,
}

var<push_constant> inverseWindowDimensions: vec2f;

@group(0) @binding({{input_texture_binding}}) var inputColorTexture: texture_2d<f32>;
@group(0) @binding({{input_sampler_binding}}) var inputColorSampler: sampler;

fn convertFramebufferPositionToScreenTextureCoords(framebufferPosition: vec4f) -> vec2f {
    return (framebufferPosition.xy * inverseWindowDimensions);
}

@vertex
fn mainVS(@location({{position_location}}) modelSpacePosition: vec3f) -> VertexOutput {
    var output: VertexOutput;
    output.projectedPosition = vec4f(modelSpacePosition, 1.0);
    return output;
}

@fragment
fn mainFS(input: VertexOutput) -> FragmentOutput {
    var output: FragmentOutput;
    let textureCoords = convertFramebufferPositionToScreenTextureCoords(input.projectedPosition);
    output.color = textureSampleLevel(inputColorTexture, inputColorSampler, textureCoords, 0.0);
    return output;
}
