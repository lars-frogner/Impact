// Adapted from https://learnopengl.com/Guest-Articles/2022/Phys.-Based-Bloom

struct VertexOutput {
    @builtin(position) projectedPosition: vec4f,
}

struct FragmentOutput {
    @location(0) color: vec4f,
}

// The output texture should be the next lower mip level (twice the size)
// relative to the input texture
var<push_constant> inverseOutputTextureDimensions: vec2f;

@group(0) @binding({{input_texture_binding}}) var inputTexture: texture_2d<f32>;
@group(0) @binding({{input_sampler_binding}}) var inputSampler: sampler;

// The width of the blur filter, in texture coordinates
const R: f32 = {{blur_filter_radius}};

@vertex
fn mainVS(@location({{position_location}}) modelSpacePosition: vec3f) -> VertexOutput {
    var output: VertexOutput;
    output.projectedPosition = vec4f(modelSpacePosition, 1.0);
    return output;
}

@fragment
fn mainFS(input: VertexOutput) -> FragmentOutput {
    var output: FragmentOutput;

    // Current coordinates in input texture
    let texCoords = input.projectedPosition.xy * inverseOutputTextureDimensions;

    // Take 9 samples around current texel `e`:
    // a - b - c
    // d - e - f
    // g - h - i
    let a = textureSampleLevel(inputTexture, inputSampler, vec2f(texCoords.x - R, texCoords.y + R), 0.0).rgb;
    let b = textureSampleLevel(inputTexture, inputSampler, vec2f(texCoords.x,     texCoords.y + R), 0.0).rgb;
    let c = textureSampleLevel(inputTexture, inputSampler, vec2f(texCoords.x + R, texCoords.y + R), 0.0).rgb;

    let d = textureSampleLevel(inputTexture, inputSampler, vec2f(texCoords.x - R, texCoords.y    ), 0.0).rgb;
    let e = textureSampleLevel(inputTexture, inputSampler, vec2f(texCoords.x,     texCoords.y    ), 0.0).rgb;
    let f = textureSampleLevel(inputTexture, inputSampler, vec2f(texCoords.x + R, texCoords.y    ), 0.0).rgb;

    let g = textureSampleLevel(inputTexture, inputSampler, vec2f(texCoords.x - R, texCoords.y - R), 0.0).rgb;
    let h = textureSampleLevel(inputTexture, inputSampler, vec2f(texCoords.x,     texCoords.y - R), 0.0).rgb;
    let i = textureSampleLevel(inputTexture, inputSampler, vec2f(texCoords.x + R, texCoords.y - R), 0.0).rgb;

    // Combine samples in using a 3x3 tent filter:
    //  1   | 1 2 1 |
    // -- * | 2 4 2 |
    // 16   | 1 2 1 |
    let averagedColor = 0.25 * e + 0.125 * (b + d + f + h) + 0.0625 * (a + c + g + i);

    output.color = vec4f(averagedColor, 1.0);
    return output;
}
