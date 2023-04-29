struct AmbientOcclusionSampleRotation {
    cosRandomAngle: f32,
    sinRandomAngle: f32,
}

fn computeAmbientOcclusionSampleRotation(
    randomAngle: f32,
) -> AmbientOcclusionSampleRotation {
    var rotation: AmbientOcclusionSampleRotation;

    rotation.cosRandomAngle = cos(randomAngle);
    rotation.sinRandomAngle = sin(randomAngle);
    
    return rotation;
}

fn computeAmbientOcclusionSampleValue(
    positionTexture: texture_2d<f32>,
    positionSampler: sampler,
    projectionMatrix: mat4x4<f32>,
    squaredSampleRadius: f32,
    position: vec3<f32>,
    normalVector: vec3<f32>,
    rotation: AmbientOcclusionSampleRotation,
    sample: vec4<f32>,
) -> f32 {
    // Rotate horizontal sample offset by random angle
    let rotatedSampleOffset = vec2<f32>(
        sample.x * rotation.cosRandomAngle - sample.y * rotation.sinRandomAngle,
        sample.x * rotation.sinRandomAngle + sample.y * rotation.cosRandomAngle,
    );

    // Calculate view space sampling position (using depth of sphere center as
    // sample depth, which is only needed for the projection to texture
    // coordinates)
    let samplingPosition = vec3<f32>(position.xy + rotatedSampleOffset, position.z);

    // Convert sampling position to texture coordinates for the render
    // attachment textures by projecting to clip space with the camera
    // projection
    let sampleTextureCoords = computeTextureCoordsForAmbientOcclusionSample(projectionMatrix, samplingPosition);

    // Sample view space position of the occluder closest to the camera at the
    // projected position
    let sampledOccluderPosition = textureSampleLevel(positionTexture, positionSampler, sampleTextureCoords, 0.0).xyz;

    // Compute vector from fragment position to occluder position
    let sampledOccluderDisplacement = sampledOccluderPosition - position;

    let sampledOccluderDisplacementAlongNormal = dot(sampledOccluderDisplacement, normalVector);
    let squaredSampledOccluderDistance = dot(sampledOccluderDisplacement, sampledOccluderDisplacement);

    // Include a small bias distance to avoid self-shadowing
    let biasDistance = 1e-4 * position.z;

    // We may want to exclude occluders outside the sampling sphere
    let isWithinSampleRadius = 1.0; // step(squaredSampledOccluderDistance, squaredSampleRadius);

    // Compute sample for the visibility estimator of McGuire et al. (2011),
    // "The Alchemy Screen-Space Ambient Obscurance Algorithm"
    return max(0.0, (sampledOccluderDisplacementAlongNormal + biasDistance) * isWithinSampleRadius) / (squaredSampledOccluderDistance + 1e-4);
}

fn computeTextureCoordsForAmbientOcclusionSample(
    projectionMatrix: mat4x4<f32>,
    samplingPosition: vec3<f32>,
) -> vec2<f32> {
    let undividedClipSpaceSamplingPosition = projectionMatrix * vec4<f32>(samplingPosition, 1.0);
    let horizontalClipSpaceSamplingPosition = undividedClipSpaceSamplingPosition.xy / undividedClipSpaceSamplingPosition.w;
    var sampleTextureCoords = 0.5 * (horizontalClipSpaceSamplingPosition + 1.0);
    sampleTextureCoords.y = 1.0 - sampleTextureCoords.y;
    return sampleTextureCoords;
}

// Evaluates the visibility estimator of McGuire et al. (2011), "The Alchemy
// Screen-Space Ambient Obscurance Algorithm"
fn computeAmbientVisibility(sampleNormalization: f32, contrast: f32, summedSampleValues: f32) -> f32 {
    return pow(max(0.0, 1.0 - sampleNormalization * summedSampleValues), contrast);
}

fn computeOccludedAmbientColor(
    positionTexture: texture_2d<f32>,
    positionSampler: sampler,
    occlusionTexture: texture_2d<f32>,
    occlusionSampler: sampler,
    texelDimensions: vec2<f32>,
    centerTextureCoords: vec2<f32>,
    ambientColor: vec3<f32>,
) -> vec3<f32> {
    // This should be odd so that the center is included
    let sqrtTotalSampleCount = 5u;

    let maxDepthDifference = 0.01;

    let halfSampleAreaDimensions = 0.5 * texelDimensions * f32(sqrtTotalSampleCount - 1u);
    let lowerTextureCoords = centerTextureCoords - halfSampleAreaDimensions;

    let centerDepth = textureSampleLevel(positionTexture, positionSampler, centerTextureCoords, 0.0).z;

    var summedOcclusion: f32 = 0.0;
    var acceptedSampleCount: u32 = 0u;

    for (var i: u32 = 0u; i < sqrtTotalSampleCount; i++) {
        let u = lowerTextureCoords.x + f32(i) * texelDimensions.x;
        for (var j: u32 = 0u; j < sqrtTotalSampleCount; j++) {
            let v = lowerTextureCoords.y + f32(j) * texelDimensions.y;
            let textureCoords = vec2<f32>(u, v);

            let depth = textureSampleLevel(positionTexture, positionSampler, textureCoords, 0.0).z;
            
            if abs(depth - centerDepth) < maxDepthDifference {
                summedOcclusion += textureSampleLevel(occlusionTexture, occlusionSampler, textureCoords, 0.0).r;
                acceptedSampleCount += 1u;
            }
        }
    }

    let occlusion = summedOcclusion / f32(acceptedSampleCount);

    return occlusion * ambientColor;
}
