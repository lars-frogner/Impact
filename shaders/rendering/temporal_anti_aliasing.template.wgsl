struct VertexOutput {
    @builtin(position) projectedPosition: vec4f,
}

struct FragmentOutput {
    @location(0) luminance: vec4f,
}

struct Parameters {
    currentFrameWeight: f32,
    varianceClippingThreshold: f32,
}

var<push_constant> inverseWindowDimensions: vec2f;

@group({{linear_depth_texture_group}}) @binding({{linear_depth_texture_binding}})
var linearDepthTexture: texture_2d<f32>;
@group({{linear_depth_texture_group}}) @binding({{linear_depth_sampler_binding}})
var linearDepthSampler: sampler;

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

@group({{params_group}}) @binding({{params_binding}}) var<uniform> params: Parameters;

fn convertFramebufferPositionToPixelIndices(framebufferPosition: vec4f) -> vec2i {
    return vec2i(trunc(framebufferPosition.xy));
}

fn convertFramebufferPositionToScreenTextureCoords(framebufferPosition: vec4f) -> vec2f {
    return (framebufferPosition.xy * inverseWindowDimensions);
}

// Samples the given texture using an optimized version of Catmul-Rom filtering.
// Adapted from
// https://gist.github.com/TheRealMJP/c83b8c0f46b63f3a88a5986f4fa982b1, see
// there for explanations.
fn sampleTextureCatmullRom(
    inputTexture: texture_2d<f32>,
    inputSampler: sampler,
    textureCoords: vec2f,
    textureDims: vec2f,
) -> vec3f {
    let samplePos = textureCoords * textureDims;
    let texPos1 = floor(samplePos - 0.5) + 0.5;

    let f = samplePos - texPos1;

    let w0 = f * (-0.5 + f * (1.0 - 0.5 * f));
    let w1 = 1.0 + f * f * (-2.5 + 1.5 * f);
    let w2 = f * (0.5 + f * (2.0 - 1.5 * f));
    let w3 = f * f * (-0.5 + 0.5 * f);

    let w12 = w1 + w2;
    let offset12 = w2 / (w1 + w2);

    var texPos0 = texPos1 - 1.0;
    var texPos3 = texPos1 + 2.0;
    var texPos12 = texPos1 + offset12;

    texPos0 /= textureDims;
    texPos3 /= textureDims;
    texPos12 /= textureDims;

    var result = vec3f(0.0, 0.0, 0.0);
    result += textureSampleLevel(inputTexture, inputSampler, vec2f(texPos0.x, texPos0.y), 0.0).rgb * w0.x * w0.y;
    result += textureSampleLevel(inputTexture, inputSampler, vec2f(texPos12.x, texPos0.y), 0.0).rgb * w12.x * w0.y;
    result += textureSampleLevel(inputTexture, inputSampler, vec2f(texPos3.x, texPos0.y), 0.0).rgb * w3.x * w0.y;
    result += textureSampleLevel(inputTexture, inputSampler, vec2f(texPos0.x, texPos12.y), 0.0).rgb * w0.x * w12.y;
    result += textureSampleLevel(inputTexture, inputSampler, vec2f(texPos12.x, texPos12.y), 0.0).rgb * w12.x * w12.y;
    result += textureSampleLevel(inputTexture, inputSampler, vec2f(texPos3.x, texPos12.y), 0.0).rgb * w3.x * w12.y;
    result += textureSampleLevel(inputTexture, inputSampler, vec2f(texPos0.x, texPos3.y), 0.0).rgb * w0.x * w3.y;
    result += textureSampleLevel(inputTexture, inputSampler, vec2f(texPos12.x, texPos3.y), 0.0).rgb * w12.x * w3.y;
    result += textureSampleLevel(inputTexture, inputSampler, vec2f(texPos3.x, texPos3.y), 0.0).rgb * w3.x * w3.y;

    return result;
}

fn computeMitchellNetravaliFilterWeight(distanceFromCenter: f32) -> f32 {
    // Scale the distance so that the filter effectively covers the range 
    // [-1, 1] rather than [-2, 2]
    let x = 2.0 * distanceFromCenter;

    let x2 = x * x;
    let x3 = x2 * x;
    
    var y = 0.0;

    if (x < 1.0) {
        y = 7.0 * x3 - 12.0 * x2 + 5.33333333333;
    } else if (x <= 2.0) {
        y = -2.33333333333 * x3 + 12.0 * x2 - 20.0 * x + 10.6666666667;
    }

    return y * 0.1666666667;
}

// Finds the intersection point between the given AABB and the ray from the AABB
// center to the given point. Returns the point itself if it is inside the AABB.
// Adapted from
// https://github.com/playdeadgames/temporal/blob/master/Assets/Shaders/TemporalReprojection.shader.
fn clipWithAABBTowardCenter(minCorner: vec3f, maxCorner: vec3f, point: vec3f) -> vec3f {
    let center = 0.5 * (maxCorner + minCorner);
    let halfWidths = 0.5 * (maxCorner - minCorner) + 1e-6;

    let displacement = point - center;
    let relativeDisplacement = abs(displacement / halfWidths);
    let maxRelativeDisplacement = max(relativeDisplacement.x, max(relativeDisplacement.y, relativeDisplacement.z));

    if (maxRelativeDisplacement > 1.0) {
        return center + displacement / maxRelativeDisplacement;
    } else {
        // Point is inside AABB
        return point;
    }
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
    previousLuminance: vec3f,
    luminance: vec3f,
) -> vec2f {
    let toneMappedPreviousFrameWeight = previousFrameWeight / (1.0 + computeScalarLuminanceFromColor(previousLuminance));
    let toneMappedCurrentFrameWeight = currentFrameWeight / (1.0 + computeScalarLuminanceFromColor(luminance));

    let inverseWeightSum = 1.0 / max(toneMappedPreviousFrameWeight + toneMappedCurrentFrameWeight, 1e-6);

    return vec2f(toneMappedPreviousFrameWeight, toneMappedCurrentFrameWeight) * inverseWeightSum;
}

@vertex 
fn mainVS(@location({{position_location}}) modelSpacePosition: vec3f) -> VertexOutput {
    var output: VertexOutput;
    output.projectedPosition = vec4f(modelSpacePosition, 1.0);
    return output;
}

const MAX_F32: f32 = 3.40282e38;
const MIN_F32: f32 = -3.40282e38;

const ONE_OVER_NEIGHBORHOOD_SAMPLE_COUNT: f32 = 1.0 / 9.0;

@fragment 
fn mainFS(input: VertexOutput) -> FragmentOutput {
    var output: FragmentOutput;

    let textureDims = vec2i(textureDimensions(luminanceTexture));
    let pixelIndices = convertFramebufferPositionToPixelIndices(input.projectedPosition);

    var luminance = vec3f(0.0);
    var totalLuminanceWeight = 0.0;

    var minNeighborLuminance = vec3f(MAX_F32);
    var maxNeighborLuminance = vec3f(MIN_F32);

    var luminanceFirstMoment = vec3f(0.0);
    var luminanceSecondMoment = vec3f(0.0);

    var closestDepth = 1.0;
    var closestDepthPixelIndices = vec2i(0);

    // Iterate over the pixels in the 3x3 neighborhood around the current pixel.
    //
    // To get a stable reconstruction of the current luminance, we sum up the
    // luminances in the neighborhood weighted by a Mitchell-Netravali filter
    // covering the neighborhood, and divide by the total weight after the loop.
    //
    // Since we want to clamp the previous luminance to not extend outside the
    // luminance range of the neighborhood, we also find the component-wise
    // minimum and maximum of the neighborhood luminances.
    //
    // We also keep track of the pixel with the smallest depth in the
    // neighborhood, which we use to obtain the motion vector of the pixel that
    // is most in the foreground.
    for (var i: i32 = -1; i <= 1; i++) {
        for (var j: i32 = -1; j <= 1; j++) {
            let neighborPixelIndices = clamp(pixelIndices + vec2i(i, j), vec2i(0), textureDims - 1);

            let neighborLuminance = textureLoad(luminanceTexture, neighborPixelIndices, 0).rgb;
            let neighborDistance = length(vec2f(f32(i), f32(j)));
            let neighborWeight = computeMitchellNetravaliFilterWeight(neighborDistance);

            luminance += neighborLuminance * neighborWeight;
            totalLuminanceWeight += neighborWeight;

            minNeighborLuminance = min(minNeighborLuminance, neighborLuminance);
            maxNeighborLuminance = max(maxNeighborLuminance, neighborLuminance);

            luminanceFirstMoment += neighborLuminance;
            luminanceSecondMoment += neighborLuminance * neighborLuminance;

            let neighborDepth = textureLoad(linearDepthTexture, neighborPixelIndices, 0).x;
            if (neighborDepth < closestDepth) {
                closestDepth = neighborDepth;
                closestDepthPixelIndices = neighborPixelIndices;
            }
        }
    }

    luminance /= totalLuminanceWeight;

    let motionVector = textureLoad(motionVectorTexture, closestDepthPixelIndices, 0).xy;

    let textureCoords = convertFramebufferPositionToScreenTextureCoords(input.projectedPosition);
    let previousTextureCoords = textureCoords - motionVector;

    // If the previous texture coordinates are out of bounds, we simply return
    // the luminance from the current frame
    if (any(previousTextureCoords != saturate(previousTextureCoords))) {
        output.luminance = vec4f(luminance, 1.0);
        return output;
    }
    
    var previousLuminance = sampleTextureCatmullRom(
        previousLuminanceTexture,
        previousLuminanceSampler,
        previousTextureCoords,
        vec2f(textureDims),
    );

    // Clamp the previous luminance to the range of the neighborhood
    previousLuminance = clamp(previousLuminance, minNeighborLuminance, maxNeighborLuminance);

    // Clip the previous luminance to the AABB spanning the contour of the color
    // distribution of the neighborhood
    let luminanceMean = luminanceFirstMoment * ONE_OVER_NEIGHBORHOOD_SAMPLE_COUNT;
    let luminanceStdDev = sqrt(abs(luminanceSecondMoment * ONE_OVER_NEIGHBORHOOD_SAMPLE_COUNT - luminanceMean * luminanceMean));
    let lowerClippingLuminance = luminanceMean - params.varianceClippingThreshold * luminanceStdDev;
    let upperClippingLuminance = luminanceMean + params.varianceClippingThreshold * luminanceStdDev;
    previousLuminance = clipWithAABBTowardCenter(lowerClippingLuminance, upperClippingLuminance, previousLuminance);

    var currentFrameWeight = params.currentFrameWeight;
    var previousFrameWeight = 1.0 - currentFrameWeight;

    let adjustedWeights = computeToneMappingAdjustedWeights(previousFrameWeight, currentFrameWeight, previousLuminance, luminance);
    previousFrameWeight = adjustedWeights.x;
    currentFrameWeight = adjustedWeights.y;

    let blendedLuminance = (previousFrameWeight * previousLuminance + currentFrameWeight * luminance);
    
    output.luminance = vec4f(blendedLuminance, 1.0);
    return output;
}