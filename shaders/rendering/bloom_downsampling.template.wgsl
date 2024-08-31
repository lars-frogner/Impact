// Adapted from https://learnopengl.com/Guest-Articles/2022/Phys.-Based-Bloom

struct VertexOutput {
    @builtin(position) projectedPosition: vec4f,
}

struct FragmentOutput {
    @location(0) color: vec4f,
}

// The output texture should be the next higher mip level (half the size)
// relative to the input texture
var<push_constant> inverseOutputTextureDimensions: vec2f;

@group(0) @binding({{input_texture_binding}}) var inputTexture: texture_2d<f32>;
@group(0) @binding({{input_sampler_binding}}) var inputSampler: sampler;

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

    let inputTextureDimensions = textureDimensions(inputTexture);
    let inputTexelDimensions = 1.0 / vec2f(inputTextureDimensions);

    let x = inputTexelDimensions.x;
    let y = inputTexelDimensions.y;

    // Take 13 samples around current texel `e`:
    // a - b - c
    // - j - k -
    // d - e - f
    // - l - m -
    // g - h - i
    let a = textureSampleLevel(inputTexture, inputSampler, vec2f(texCoords.x - 2.0 * x, texCoords.y + 2.0 * y), 0.0).rgb;
    let b = textureSampleLevel(inputTexture, inputSampler, vec2f(texCoords.x,           texCoords.y + 2.0 * y), 0.0).rgb;
    let c = textureSampleLevel(inputTexture, inputSampler, vec2f(texCoords.x + 2.0 * x, texCoords.y + 2.0 * y), 0.0).rgb;

    let d = textureSampleLevel(inputTexture, inputSampler, vec2f(texCoords.x - 2.0 * x, texCoords.y), 0.0).rgb;
    let e = textureSampleLevel(inputTexture, inputSampler, vec2f(texCoords.x,           texCoords.y), 0.0).rgb;
    let f = textureSampleLevel(inputTexture, inputSampler, vec2f(texCoords.x + 2.0 * x, texCoords.y), 0.0).rgb;

    let g = textureSampleLevel(inputTexture, inputSampler, vec2f(texCoords.x - 2.0 * x, texCoords.y - 2.0 * y), 0.0).rgb;
    let h = textureSampleLevel(inputTexture, inputSampler, vec2f(texCoords.x,           texCoords.y - 2.0 * y), 0.0).rgb;
    let i = textureSampleLevel(inputTexture, inputSampler, vec2f(texCoords.x + 2.0 * x, texCoords.y - 2.0 * y), 0.0).rgb;

    let j = textureSampleLevel(inputTexture, inputSampler, vec2f(texCoords.x - x, texCoords.y + y), 0.0).rgb;
    let k = textureSampleLevel(inputTexture, inputSampler, vec2f(texCoords.x + x, texCoords.y + y), 0.0).rgb;
    let l = textureSampleLevel(inputTexture, inputSampler, vec2f(texCoords.x - x, texCoords.y - y), 0.0).rgb;
    let m = textureSampleLevel(inputTexture, inputSampler, vec2f(texCoords.x + x, texCoords.y - y), 0.0).rgb;

    // Combine samples in 5 overlapping groups with associated weights:
    // (j, k, l, m): 0.5
    // (a, b, d, e): 0.125
    // (b, c, e, f): 0.125
    // (d, e, g, h): 0.125
    // (e, f, h, i): 0.125

    // Summing and normalizing the sum of weigths to unity yields:
    let averagedColor = 0.125 * (e + j + k + l + m) + 0.0625 * (b + d + f + h) + 0.03125 * (a + c + g + i);

    output.color = vec4f(averagedColor, 1.0);
    return output;
}
