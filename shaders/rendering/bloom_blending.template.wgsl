struct VertexOutput {
    @builtin(position) projectedPosition: vec4f,
}

struct FragmentOutput {
    @location(0) blendedLuminance: vec4f,
}

var<push_constant> inverseWindowDimensions: vec2f;

@group({{luminance_texture_group}}) @binding({{luminance_texture_binding}})
var luminanceTexture: texture_2d<f32>;
@group({{luminance_texture_group}}) @binding({{luminance_sampler_binding}})
var luminanceSampler: sampler;

@group({{blurred_luminance_texture_group}}) @binding({{blurred_luminance_texture_binding}})
var blurredLuminanceTexture: texture_2d<f32>;
@group({{blurred_luminance_texture_group}}) @binding({{blurred_luminance_sampler_binding}})
var blurredLuminanceSampler: sampler;

const BLURRED_LUMINANCE_NORMALIZATION: f32 = {{blurred_luminance_normalization}};
const BLURRED_LUMINANCE_WEIGHT: f32 = {{blurred_luminance_weight}};
const LUMINANCE_WEIGHT: f32 = 1.0 - BLURRED_LUMINANCE_WEIGHT;
const NORMALIZED_BLURRED_LUMINANCE_WEIGHT: f32 = BLURRED_LUMINANCE_NORMALIZATION * BLURRED_LUMINANCE_WEIGHT;

fn convertFramebufferPositionToScreenTextureCoords(framebufferPosition: vec4f) -> vec2f {
    return framebufferPosition.xy * inverseWindowDimensions;
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

    let luminance = textureSampleLevel(luminanceTexture, luminanceSampler, textureCoords, 0.0).rgb;
    let blurredLuminance = textureSampleLevel(blurredLuminanceTexture, blurredLuminanceSampler, textureCoords, 0.0).rgb;

    let blendedLuminance = NORMALIZED_BLURRED_LUMINANCE_WEIGHT * blurredLuminance + LUMINANCE_WEIGHT * luminance;

    output.blendedLuminance = vec4f(blendedLuminance, 1.0);
    return output;
}
