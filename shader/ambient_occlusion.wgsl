fn computeAmbientOcclusionMax64Samples(
    positionTexture: texture_2d<f32>,
    positionSampler: sampler,
    sampleOffsets: array<vec4<f32>, 32u>,
    sampleCount: u32,
    position: vec3<f32>,
    normalVector: vec3<f32>,
    noiseFactor: f32,
) -> f32 {
    return 1.0;
}

fn computeOccludedAmbientColor(
    occlusionTexture: texture_2d<f32>,
    occlusionSampler: sampler,
    centerTextureCoords: vec2<f32>,
    ambientColor: vec3<f32>,
    noiseFactor: f32,
) -> vec3<f32> {
    return textureSample(occlusionTexture, occlusionSampler, centerTextureCoords).r * ambientColor;
}
