struct AmbientOcclusionSamplingSpace {
    sphereCenter: vec3<f32>,
    cosRandomAngle: f32,
    sinRandomAngle: f32,
}

fn computeAmbientOcclusionSamplingSpace(
    sampleRadius: f32,
    position: vec3<f32>,
    normalVector: vec3<f32>,
    randomAngle: f32,
) -> AmbientOcclusionSamplingSpace {
    var space: AmbientOcclusionSamplingSpace;

    // The center of the sampling sphere is offset from the fragment position
    // along the fragment normal by a distance equal to its radius 
    space.sphereCenter = position + sampleRadius * normalVector;
    
    space.cosRandomAngle = cos(randomAngle);
    space.sinRandomAngle = sin(randomAngle);
    
    return space;
}

fn computeUnoccupiedHeightForSample(
    positionTexture: texture_2d<f32>,
    positionSampler: sampler,
    projectionMatrix: mat4x4<f32>,
    space: AmbientOcclusionSamplingSpace,
    sample: vec4<f32>,
) -> f32 {
    // Rotate horizontal sample offset by random angle
    let rotatedSampleOffset = vec2<f32>(
        sample.x * space.cosRandomAngle - sample.y * space.sinRandomAngle,
        sample.x * space.sinRandomAngle + sample.y * space.cosRandomAngle,
    );

    // Calculate view space sampling position (using depth of sphere center as
    // sample depth, which is only needed for the projection to texture
    // coordinates)
    let samplePosition = vec3<f32>(space.sphereCenter.xy + rotatedSampleOffset, space.sphereCenter.z);

    // Convert sampling position to texture coordinates for the render
    // attachment textures by projecting to clip space with the camera
    // projection
    let sampleTextureCoords = computeTextureCoordsForAmbientOcclusionSample(projectionMatrix, samplePosition);

    // Sample view space depth of the geometry at the sample position
    let depthAtSamplePosition = textureSampleLevel(positionTexture, positionSampler, sampleTextureCoords, 0.0).z;

    let halfSphereHeight = sample.z;
    let sphereHeight = sample.w;

    // Calculate the vertical distance within the sphere that is occupied by
    // geometry at the sample position
    let unoccupiedHeight = clamp(halfSphereHeight + space.sphereCenter.z - depthAtSamplePosition, 0.0, sphereHeight);

    return unoccupiedHeight;
}

fn computeTextureCoordsForAmbientOcclusionSample(
    projectionMatrix: mat4x4<f32>,
    samplePosition: vec3<f32>,
) -> vec2<f32> {
    let undividedClipSpaceSamplePosition = projectionMatrix * vec4<f32>(samplePosition, 1.0);
    let horizontalClipSpaceSamplePosition = undividedClipSpaceSamplePosition.xy / undividedClipSpaceSamplePosition.w;
    var sampleTextureCoords = 0.5 * (horizontalClipSpaceSamplePosition + 1.0);
    sampleTextureCoords.y = 1.0 - sampleTextureCoords.y;
    return sampleTextureCoords;
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
