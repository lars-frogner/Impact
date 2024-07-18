struct VertexOutput {
    @builtin(position) projectedPosition: vec4<f32>,
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
}

var<push_constant> inverseWindowDimensions: vec2<f32>;

@group(0) @binding({{input_texture_binding}}) var inputColorTexture: texture_2d<f32>;
@group(0) @binding({{input_sampler_binding}}) var inputColorSampler: sampler;

fn convertFramebufferPositionToScreenTextureCoords(framebufferPosition: vec4<f32>) -> vec2<f32> {
    return (framebufferPosition.xy * inverseWindowDimensions);
}

@vertex 
fn mainVS(@location({{position_location}}) modelSpacePosition: vec3<f32>) -> VertexOutput {
    var output: VertexOutput;
    output.projectedPosition = vec4<f32>(modelSpacePosition, 1.0);
    return output;
}

@fragment 
fn mainFS(input: VertexOutput) -> FragmentOutput {
    var output: FragmentOutput;
    let textureCoords = convertFramebufferPositionToScreenTextureCoords(input.projectedPosition);
    output.color = textureSampleLevel(inputColorTexture, inputColorSampler, textureCoords, 0.0);
    return output;
}