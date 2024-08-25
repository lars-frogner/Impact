struct GaussianBlurSamples {
    sampleOffsetsAndWeights: array<vec4f, {{max_samples}}>,
    sampleCount: u32,
}

struct VertexOutput {
    @builtin(position) projectedPosition: vec4f,
}

struct FragmentOutput {
    @location(0) color: vec4f,
}

var<push_constant> inverseWindowDimensions: vec2f;

@group({{input_texture_group}}) @binding({{input_texture_binding}}) var inputTexture: texture_2d<f32>;
@group({{input_texture_group}}) @binding({{input_sampler_binding}}) var inputSampler: sampler;

@group({{samples_group}}) @binding({{samples_binding}}) var<uniform> gaussianBlurSamples: GaussianBlurSamples;

fn computeSingleGaussianBlurSampleColor(fragmentCoords: vec2f, offset: f32, weight: f32) -> vec4f {
    return compute{{direction}}GaussianBlurSampleColor(fragmentCoords, offset, weight);
}

fn computeSymmetricGaussianBlurSampleColor(fragmentCoords: vec2f, offset: f32, weight: f32) -> vec4f {
    let positiveOffsetSampleColor = compute{{direction}}GaussianBlurSampleColor(fragmentCoords, offset, weight);
    let negativeOffsetSampleColor = compute{{direction}}GaussianBlurSampleColor(fragmentCoords, -offset, weight);
    return positiveOffsetSampleColor + negativeOffsetSampleColor;
}

fn computeHorizontalGaussianBlurSampleColor(fragmentCoords: vec2f, offset: f32, weight: f32) -> vec4f {
    let sampleTextureCoords = (fragmentCoords + vec2(offset, 0.0)) * inverseWindowDimensions;
    let sampledInputColor = textureSampleLevel(inputTexture, inputSampler, sampleTextureCoords, 0.0);
    return sampledInputColor * weight;
}

fn computeVerticalGaussianBlurSampleColor(fragmentCoords: vec2f, offset: f32, weight: f32) -> vec4f {
    let sampleTextureCoords = (fragmentCoords + vec2(0.0, offset)) * inverseWindowDimensions;
    let sampledInputColor = textureSampleLevel(inputTexture, inputSampler, sampleTextureCoords, 0.0);
    return sampledInputColor * weight;
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

    let fragmentCoords = input.projectedPosition.xy;

    let firstOffsetAndWeight = gaussianBlurSamples.sampleOffsetsAndWeights[0u];
    output.color = computeSingleGaussianBlurSampleColor(fragmentCoords, firstOffsetAndWeight.x, firstOffsetAndWeight.y);

    for (var sampleIdx: u32 = 1u; sampleIdx < gaussianBlurSamples.sampleCount; sampleIdx++) {
        let offsetAndWeight = gaussianBlurSamples.sampleOffsetsAndWeights[sampleIdx];
        output.color += computeSymmetricGaussianBlurSampleColor(fragmentCoords, offsetAndWeight.x, offsetAndWeight.y);
    }

    return output;
}
