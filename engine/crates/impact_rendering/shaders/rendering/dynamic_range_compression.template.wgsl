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

fn toneMapNone(rgbaColor: vec4f) -> vec4f {
    return rgbaColor;
}

fn toneMapACES(rgbaColor: vec4f) -> vec4f {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    var color = rgbaColor.rgb;
    color *= 0.6;
    color = saturate((color * (a * color + b)) / (color * (c * color + d) + e));
    return vec4f(color, rgbaColor.a);
}

fn toneMapKhronosPBRNeutral(rgbaColor: vec4f) -> vec4f {
    let startCompression = 0.8 - 0.04;
    let desaturation = 0.15;

    var color = rgbaColor.rgb;

    let x = min(color.r, min(color.g, color.b));
    let offset = select(x - 6.25 * x * x, 0.04, x < 0.08);
    color -= offset;

    let peak = max(color.r, max(color.g, color.b));
    if (peak < startCompression) {
        return vec4f(color, rgbaColor.a);
    }

    let d = 1.0 - startCompression;
    let newPeak = 1.0 - d * d / (peak + d - startCompression);
    color *= newPeak / peak;

    let g = 1.0 - 1.0 / (desaturation * (peak - newPeak) + 1.0);
    color = mix(color, newPeak * vec3(1.0, 1.0, 1.0), g);

    return vec4f(color, rgbaColor.a);
}

fn linearToSrgb(rgbaColor: vec4f) -> vec4f {
    var color = rgbaColor.rgb;
    let low = color * 12.92;
    let high = (pow(abs(color), vec3f(1.0 / 2.4)) * 1.055) - 0.055;
    color = select(high, low, color <= vec3f(0.0031308));
    return vec4f(color, rgbaColor.a);
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
    let rgbaColor = textureSampleLevel(inputColorTexture, inputColorSampler, textureCoords, 0.0);
    output.color = toneMap{{tone_mapping_method}}(rgbaColor);
    output.color = linearToSrgb(output.color);
    return output;
}
