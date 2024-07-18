struct VertexOutput {
    @builtin(position) projectedPosition: vec4<f32>,
}

struct FragmentOutput {
    @location(0) luminance: vec4<f32>,
}

var<push_constant> inverseWindowDimensions: vec2<f32>;

@group({{position_texture_group}}) @binding({{position_texture_binding}})
var positionTexture: texture_2d<f32>;
@group({{position_texture_group}}) @binding({{position_sampler_binding}})
var positionSampler: sampler;

@group({{ambient_reflected_luminance_texture_group}}) @binding({{ambient_reflected_luminance_texture_binding}})
var ambientReflectedLuminanceTexture: texture_2d<f32>;
@group({{ambient_reflected_luminance_texture_group}}) @binding({{ambient_reflected_luminance_sampler_binding}})
var ambientReflectedLuminanceSampler: sampler;

@group({{occlusion_texture_group}}) @binding({{occlusion_texture_binding}})
var occlusionTexture: texture_2d<f32>;
@group({{occlusion_texture_group}}) @binding({{occlusion_sampler_binding}})
var occlusionSampler: sampler;

fn convertFramebufferPositionToScreenTextureCoords(framebufferPosition: vec4<f32>) -> vec2<f32> {
    return (framebufferPosition.xy * inverseWindowDimensions);
}

fn computeOccludedAmbientReflectedLuminance(
    centerTextureCoords: vec2<f32>,
    ambientReflectedLuminance: vec3<f32>,
) -> vec3<f32> {
    // This should be odd so that the center is included
    let sqrtTotalSampleCount = 5u;

    let maxDepthDifference = 0.01;

    let halfSampleAreaDimensions = 0.5 * inverseWindowDimensions * f32(sqrtTotalSampleCount - 1u);
    let lowerTextureCoords = centerTextureCoords - halfSampleAreaDimensions;

    let centerDepth = textureSampleLevel(positionTexture, positionSampler, centerTextureCoords, 0.0).z;

    var summedOcclusion: f32 = 0.0;
    var acceptedSampleCount: u32 = 0u;

    for (var i: u32 = 0u; i < sqrtTotalSampleCount; i++) {
        let u = lowerTextureCoords.x + f32(i) * inverseWindowDimensions.x;
        for (var j: u32 = 0u; j < sqrtTotalSampleCount; j++) {
            let v = lowerTextureCoords.y + f32(j) * inverseWindowDimensions.y;
            let textureCoords = vec2<f32>(u, v);

            let depth = textureSampleLevel(positionTexture, positionSampler, textureCoords, 0.0).z;
            
            if abs(depth - centerDepth) < maxDepthDifference {
                summedOcclusion += textureSampleLevel(occlusionTexture, occlusionSampler, textureCoords, 0.0).r;
                acceptedSampleCount += 1u;
            }
        }
    }

    let occlusion = summedOcclusion / f32(acceptedSampleCount);

    return occlusion * ambientReflectedLuminance;
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
    let ambientReflectedLuminance = textureSampleLevel(ambientReflectedLuminanceTexture, ambientReflectedLuminanceSampler, textureCoords, 0.0);
    let occludedAmbientReflectedLuminance = computeOccludedAmbientReflectedLuminance(textureCoords, ambientReflectedLuminance.rgb);
    output.luminance = vec4<f32>(occludedAmbientReflectedLuminance, 1.0);
    return output;
}