fn scaleColorWithExposure(rgbaColor: vec4<f32>, exposure: f32) -> vec4<f32> {
    return vec4<f32>(rgbaColor.rgb * exposure, rgbaColor.a);
}

fn toneMapACES(rgbaColor: vec4<f32>) -> vec4<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    var color = rgbaColor.rgb;
    color *= 0.6;
    color = saturate((color * (a * color + b)) / (color * (c * color + d) + e));
    return vec4<f32>(color, rgbaColor.a);
}

fn toneMapKhronosPBRNeutral(rgbaColor: vec4<f32>) -> vec4<f32> {
    let startCompression = 0.8 - 0.04;
    let desaturation = 0.15;

    var color = rgbaColor.rgb;

    let x = min(color.r, min(color.g, color.b));
    let offset = select(x - 6.25 * x * x, 0.04, x < 0.08);
    color -= offset;

    let peak = max(color.r, max(color.g, color.b));
    if (peak < startCompression) {
        return vec4<f32>(color, rgbaColor.a);
    }

    let d = 1.0 - startCompression;
    let newPeak = 1.0 - d * d / (peak + d - startCompression);
    color *= newPeak / peak;

    let g = 1.0 - 1.0 / (desaturation * (peak - newPeak) + 1.0);
    color = mix(color, newPeak * vec3(1.0, 1.0, 1.0), g);

    return vec4<f32>(color, rgbaColor.a);
}