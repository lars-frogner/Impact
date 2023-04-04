fn convertNormalMapColorToNormalVector(color: vec3<f32>) -> vec3<f32> {
    // May require normalization depending on filtering
    return 2.0 * (color - 0.5);
}

fn computeParallaxMappedTextureCoordinates(
    heightTexture: texture_2d<f32>,
    heightSampler: sampler,
    displacementScale: f32,
    originalTextureCoords: vec2<f32>,
    tangentToCameraSpaceRotationQuaternion: vec4<f32>,
    cameraSpaceViewDirection: vec3<f32>,
) -> vec2<f32> {
    let tangentSpaceViewDirection: vec3<f32> = rotateVectorWithInverseOfQuaternion(tangentToCameraSpaceRotationQuaternion, cameraSpaceViewDirection);

    var parallaxMappedTextureCoords: vec2<f32>;

    if tangentSpaceViewDirection.z > 0.0 {
        // Mip level must be explicit since it can not be computed automatically
        // inside non-uniform control flow
        let mipLevel = 0.0;

        let maxLayerCount = mix(64.0, 8.0, max(0.0, tangentSpaceViewDirection.z));
        let layerDepth = displacementScale / maxLayerCount;

        let textureCoordOffsetVector = tangentSpaceViewDirection.xy * (layerDepth / tangentSpaceViewDirection.z);

        var currentLayerCount = 0.0;
        var currentDepth = 0.0;
        var prevTextureCoords = originalTextureCoords;
        var currentTextureCoords = originalTextureCoords;

        let sampledHeight = textureSampleLevel(heightTexture, heightSampler, currentTextureCoords, mipLevel).r;
        var currentSampledDepth = (1.0 - sampledHeight) * displacementScale;
        var prevSampledDepth = currentSampledDepth;

        while currentSampledDepth > currentDepth && currentLayerCount < maxLayerCount {
            prevTextureCoords = currentTextureCoords;
            prevSampledDepth = currentSampledDepth;

            currentTextureCoords -= textureCoordOffsetVector;
            currentDepth += layerDepth;

            let sampledHeight = textureSampleLevel(heightTexture, heightSampler, currentTextureCoords, mipLevel).r;
            currentSampledDepth = (1.0 - sampledHeight) * displacementScale;

            currentLayerCount += 1.0;
        }

        let currentDepthDiff = currentSampledDepth - currentDepth;
        let prevDepthDiff = prevSampledDepth - (currentDepth - layerDepth);

        let interpWeightForZeroDepthDiff = currentDepthDiff / (currentDepthDiff - prevDepthDiff);

        parallaxMappedTextureCoords = mix(currentTextureCoords, prevTextureCoords, interpWeightForZeroDepthDiff);
    } else {
        parallaxMappedTextureCoords = originalTextureCoords;
    }

    return parallaxMappedTextureCoords;
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
