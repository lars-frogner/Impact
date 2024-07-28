struct VertexOutput {
    @builtin(position) projectedPosition: vec4<f32>,
}

struct FragmentOutput {
    @location(0) luminance: vec4<f32>,
}

struct Parameters {
    currentFrameWeight: vec4<f32>,
}

var<push_constant> inverseWindowDimensions: vec2<f32>;

@group({{linear_depth_texture_group}}) @binding({{linear_depth_texture_binding}})
var linearDepthTexture: texture_2d<f32>;
@group({{linear_depth_texture_group}}) @binding({{linear_depth_sampler_binding}})
var linearDepthSampler: sampler;

@group({{previous_linear_depth_texture_group}}) @binding({{previous_linear_depth_texture_binding}})
var previousLinearDepthTexture: texture_2d<f32>;
@group({{previous_linear_depth_texture_group}}) @binding({{previous_linear_depth_sampler_binding}})
var previousLinearDepthSampler: sampler;

@group({{luminance_texture_group}}) @binding({{luminance_texture_binding}})
var luminanceTexture: texture_2d<f32>;
@group({{luminance_texture_group}}) @binding({{luminance_sampler_binding}})
var luminanceSampler: sampler;

@group({{previous_luminance_texture_group}}) @binding({{previous_luminance_texture_binding}})
var previousLuminanceTexture: texture_2d<f32>;
@group({{previous_luminance_texture_group}}) @binding({{previous_luminance_sampler_binding}})
var previousLuminanceSampler: sampler;

@group({{motion_vector_texture_group}}) @binding({{motion_vector_texture_binding}})
var motionVectorTexture: texture_2d<f32>;
@group({{motion_vector_texture_group}}) @binding({{motion_vector_sampler_binding}})
var motionVectorSampler: sampler;

@group({{previous_motion_vector_texture_group}}) @binding({{previous_motion_vector_texture_binding}})
var previousMotionVectorTexture: texture_2d<f32>;
@group({{previous_motion_vector_texture_group}}) @binding({{previous_motion_vector_sampler_binding}})
var previousMotionVectorSampler: sampler;

@group({{params_group}}) @binding({{params_binding}}) var<uniform> params: Parameters;

fn convertFramebufferPositionToScreenTextureCoords(framebufferPosition: vec4<f32>) -> vec2<f32> {
    return (framebufferPosition.xy * inverseWindowDimensions);
}

const SRGB_TO_LUMINANCE: vec3f = vec3f(0.2125, 0.7154, 0.0721);

fn computeScalarLuminanceFromColor(color: vec3f) -> f32 {
    return dot(SRGB_TO_LUMINANCE, color);
}

// Adjust weights to perform tone mapping. This is equivalent to applying T(c) =
// c / (1 + luma(c)) to the input luminance colors and applying the inverse to
// the blended output color.
fn computeToneMappingAdjustedWeights(
    previousFrameWeight: f32,
    currentFrameWeight: f32,
    previousLuminance: vec3<f32>,
    luminance: vec3<f32>,
) -> vec2<f32> {
    let toneMappedPreviousFrameWeight = previousFrameWeight / (1.0 + computeScalarLuminanceFromColor(previousLuminance));
    let toneMappedCurrentFrameWeight = currentFrameWeight / (1.0 + computeScalarLuminanceFromColor(luminance));

    let inverseWeightSum = 1.0 / (toneMappedPreviousFrameWeight + toneMappedCurrentFrameWeight);

    return vec2<f32>(toneMappedPreviousFrameWeight, toneMappedCurrentFrameWeight) * inverseWeightSum;
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
    
    let luminance = textureSampleLevel(luminanceTexture, luminanceSampler, textureCoords, 0.0).rgb;
    // let depth = textureSampleLevel(linearDepthTexture, linearDepthSampler, textureCoords, 0.0).x;
    let motionVector = textureSampleLevel(motionVectorTexture, motionVectorSampler, textureCoords, 0.0).xy;

    let previousTextureCoords = textureCoords + motionVector;
    
    // let previousDepth = textureSampleLevel(linearDepthTexture, linearDepthSampler, previousTextureCoords, 0.0).x;
    // let previousMotionVector = textureSampleLevel(previousMotionVectorTexture, previousMotionVectorSampler, previousTextureCoords, 0.0).xy;
    let previousLuminance = textureSampleLevel(previousLuminanceTexture, previousLuminanceSampler, previousTextureCoords, 0.0).rgb;

    var currentFrameWeight = params.currentFrameWeight.x;

    // if previousDepth - depth > 1e-6 {
    //     currentFrameWeight = 1.0;
    // }
    
    var previousFrameWeight = 1.0 - currentFrameWeight;

    let adjustedWeights = computeToneMappingAdjustedWeights(previousFrameWeight, currentFrameWeight, previousLuminance, luminance);
    previousFrameWeight = adjustedWeights.x;
    currentFrameWeight = adjustedWeights.y;

    let blendedLuminance = (previousFrameWeight * previousLuminance + currentFrameWeight * luminance);
    
    output.luminance = vec4<f32>(blendedLuminance, 1.0);
    return output;
}