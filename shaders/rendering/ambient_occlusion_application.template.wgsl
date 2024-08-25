struct VertexOutput {
    @builtin(position) projectedPosition: vec4f,
}

struct FragmentOutput {
    @location(0) luminance: vec4f,
}

var<push_constant> inverseWindowDimensions: vec2f;

@group({{linear_depth_texture_group}}) @binding({{linear_depth_texture_binding}})
var linearDepthTexture: texture_2d<f32>;
@group({{linear_depth_texture_group}}) @binding({{linear_depth_sampler_binding}})
var linearDepthSampler: sampler;

@group({{ambient_reflected_luminance_texture_group}}) @binding({{ambient_reflected_luminance_texture_binding}})
var ambientReflectedLuminanceTexture: texture_2d<f32>;
@group({{ambient_reflected_luminance_texture_group}}) @binding({{ambient_reflected_luminance_sampler_binding}})
var ambientReflectedLuminanceSampler: sampler;

@group({{occlusion_texture_group}}) @binding({{occlusion_texture_binding}})
var occlusionTexture: texture_2d<f32>;
@group({{occlusion_texture_group}}) @binding({{occlusion_sampler_binding}})
var occlusionSampler: sampler;

fn convertFramebufferPositionToScreenTextureCoords(framebufferPosition: vec4f) -> vec2f {
    return (framebufferPosition.xy * inverseWindowDimensions);
}

fn computeOccludedAmbientReflectedLuminance(
    centerTextureCoords: vec2f,
    ambientReflectedLuminance: vec3f,
) -> vec3f {
    // This should be odd so that the center is included
    let sqrtTotalSampleCount = 5u;

    let maxDepthDifference = 1e-4;

    let halfSampleAreaDimensions = 0.5 * inverseWindowDimensions * f32(sqrtTotalSampleCount - 1u);
    let lowerTextureCoords = centerTextureCoords - halfSampleAreaDimensions;

    let centerDepth = textureSampleLevel(linearDepthTexture, linearDepthSampler, centerTextureCoords, 0.0).x;

    var summedOcclusion: f32 = 0.0;
    var acceptedSampleCount: u32 = 0u;

    for (var i: u32 = 0u; i < sqrtTotalSampleCount; i++) {
        let u = lowerTextureCoords.x + f32(i) * inverseWindowDimensions.x;
        for (var j: u32 = 0u; j < sqrtTotalSampleCount; j++) {
            let v = lowerTextureCoords.y + f32(j) * inverseWindowDimensions.y;
            let textureCoords = vec2f(u, v);

            let depth = textureSampleLevel(linearDepthTexture, linearDepthSampler, textureCoords, 0.0).x;

            if abs(depth - centerDepth) < maxDepthDifference {
                summedOcclusion += textureSampleLevel(occlusionTexture, occlusionSampler, textureCoords, 0.0).r;
                acceptedSampleCount += 1u;
            }
        }
    }

    let occlusion = summedOcclusion / max(1.0, f32(acceptedSampleCount));

    return occlusion * ambientReflectedLuminance;
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
    let ambientReflectedLuminance = textureSampleLevel(ambientReflectedLuminanceTexture, ambientReflectedLuminanceSampler, textureCoords, 0.0);
    let occludedAmbientReflectedLuminance = computeOccludedAmbientReflectedLuminance(textureCoords, ambientReflectedLuminance.rgb);
    output.luminance = vec4f(occludedAmbientReflectedLuminance, 1.0);
    return output;
}
