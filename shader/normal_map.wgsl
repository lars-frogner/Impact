fn convertNormalMapColorToNormalVector(color: vec3<f32>) -> vec3<f32> {
    // May require normalization depending on filtering
    return 2.0 * (color - 0.5);
}

fn computeParallaxMappedTextureCoordinates(
    heightTexture: texture_2d<f32>,
    heightSampler: sampler,
    displacementScale: f32,
    originalTextureCoordinates: vec2<f32>,
    tangentToCameraSpaceRotationQuaternion: vec4<f32>,
    cameraSpaceViewDirection: vec3<f32>,
) -> vec2<f32> {
    let tangentSpaceViewDirection: vec3<f32> = rotateVectorWithInverseOfQuaternion(tangentToCameraSpaceRotationQuaternion, cameraSpaceViewDirection);

    let layerDepth = displacementScale / mix(64.0, 8.0, max(0.0, tangentSpaceViewDirection.z));

    let textureCoordOffsetVector = tangentSpaceViewDirection.xy * (layerDepth / tangentSpaceViewDirection.z);

    var currentDepth = 0.0;
    var prevTextureCoords = originalTextureCoordinates;
    var currentTextureCoords = originalTextureCoordinates;

    let sampledHeight = textureSample(heightTexture, heightSampler, currentTextureCoords).r;
    var currentSampledDepth = (1.0 - sampledHeight) * displacementScale;
    var prevSampledDepth = currentSampledDepth;

    while currentSampledDepth > currentDepth {
        prevTextureCoords = currentTextureCoords;
        prevSampledDepth = currentSampledDepth;

        currentTextureCoords -= textureCoordOffsetVector;
        currentDepth += layerDepth;

        let sampledHeight = textureSample(heightTexture, heightSampler, currentTextureCoords).r;
        currentSampledDepth = (1.0 - sampledHeight) * displacementScale;
    }

    let currentDepthDiff = currentSampledDepth - currentDepth;
    let prevDepthDiff = prevSampledDepth - (currentDepth - layerDepth);

    let interpWeightForZeroDepthDiff = currentDepthDiff / (currentDepthDiff - prevDepthDiff);

    return mix(currentTextureCoords, prevTextureCoords, interpWeightForZeroDepthDiff);
}

fn obtainNormalFromHeightMap(
    heightTexture: texture_2d<f32>,
    heightSampler: sampler,
    textureCoords: vec2<f32>,
) -> vec3<f32> {
    let textureDims = textureDimensions(heightTexture);

    let offsetU = vec2<f32>(1.0 / f32(textureDims.x), 0.0);
    let offsetV = vec2<f32>(0.0, 1.0 / f32(textureDims.y));

    let heightDownU = textureSample(heightTexture, heightSampler, textureCoords - offsetU).r;
    let heightUpU = textureSample(heightTexture, heightSampler, textureCoords + offsetU).r;
    let heightDownV = textureSample(heightTexture, heightSampler, textureCoords - offsetV).r;
    let heightUpV = textureSample(heightTexture, heightSampler, textureCoords + offsetV).r;

    return normalize(vec3<f32>(heightDownU - heightUpU, heightDownV - heightUpV, 2.0));
}
