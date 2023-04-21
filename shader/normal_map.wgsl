fn computeParallaxMappedTextureCoordinates(
    heightTexture: texture_2d<f32>,
    heightSampler: sampler,
    displacementScale: f32,
    originalTextureCoords: vec2<f32>,
    tangentToCameraSpaceRotationQuaternion: vec4<f32>,
    cameraSpaceViewDirection: vec3<f32>,
) -> vec2<f32> {
    let hardMaxMipLevel = 7.0;
    let softMaxMipLevel = 6.0;

    let tangentSpaceViewDirection: vec3<f32> = transformVectorToTangentSpace(tangentToCameraSpaceRotationQuaternion, cameraSpaceViewDirection);

    var parallaxMappedTextureCoords: vec2<f32> = originalTextureCoords;

    if tangentSpaceViewDirection.z > 0.0 {
        let mipLevel = computeLevelOfDetail(textureDimensions(heightTexture), originalTextureCoords);

        // Skip parallax mapping if the level of detail is low enough
        if mipLevel <= hardMaxMipLevel {
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

            // Ensure smooth transition between parallax mapping and no parallax
            // mapping by interpolating between original and mapped texture
            // coordinates
            if mipLevel > softMaxMipLevel {
                parallaxMappedTextureCoords = mix(parallaxMappedTextureCoords, originalTextureCoords, fract(mipLevel));
            }
        }
    }

    return parallaxMappedTextureCoords;
}

fn obtainNormalFromHeightMap(
    heightTexture: texture_2d<f32>,
    heightSampler: sampler,
    heightScale: f32,
    uvPerDistance: vec2<f32>,
    textureCoords: vec2<f32>,
    cameraSpacePosition: vec3<f32>,
) -> vec3<f32> {
    let textureDims = textureDimensions(heightTexture);

    let offsetU = vec2<f32>(1.0 / f32(textureDims.x), 0.0);
    let offsetV = vec2<f32>(0.0, 1.0 / f32(textureDims.y));

    let heightDownU = textureSample(heightTexture, heightSampler, textureCoords - offsetU).r;
    let heightUpU = textureSample(heightTexture, heightSampler, textureCoords + offsetU).r;
    let heightDownV = textureSample(heightTexture, heightSampler, textureCoords - offsetV).r;
    let heightUpV = textureSample(heightTexture, heightSampler, textureCoords + offsetV).r;

    return -normalize(vec3<f32>(
        (heightUpU - heightDownU) * heightScale * 0.5 * f32(textureDims.x) * uvPerDistance.x,
        (heightUpV - heightDownV) * heightScale * 0.5 * f32(textureDims.y) * uvPerDistance.y,
        -1.0,
    ));
}

fn computeLevelOfDetail(textureDims: vec2<u32>, textureCoords: vec2<f32>) -> f32 {
    let texelPosition = textureCoords * vec2<f32>(textureDims);
    let duvdx = dpdx(texelPosition);
    let duvdy = dpdy(texelPosition);
    let duv = duvdx * duvdx + duvdy * duvdy;
    let maxduv = max(duv.x, duv.y);
    return max(0.0, 0.5 * log2(maxduv));
}