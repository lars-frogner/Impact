// ***** Ambient lights *****

fn computeAmbientColorForLambertian(diffuseColor: vec3<f32>, ambientRadiance: vec3<f32>) -> vec3<f32> {
    return diffuseColor * ambientRadiance;
}

fn computeAmbientColorForSpecularGGX(
    specularGGXReflectanceLookupTexture: texture_2d_array<f32>,
    specularGGXReflectanceLookupSampler: sampler,
    viewDirection: vec3<f32>,
    normalVector: vec3<f32>,
    specularColor: vec3<f32>,
    roughness: f32,
    ambientRadiance: vec3<f32>,
) -> vec3<f32> {
    var ambientColor: vec3<f32>;

    let viewDirectionDotNormalVector = dot(viewDirection, normalVector);

    if viewDirectionDotNormalVector > 0.0 {
        // Mip level must be explicit since it can not be computed automatically
        // inside non-uniform control flow
        let mipLevel = 0.0;

        let textureCoords = vec2<f32>(viewDirectionDotNormalVector, roughness);

        let reflectanceForSpecularColorZero = textureSampleLevel(
            specularGGXReflectanceLookupTexture,
            specularGGXReflectanceLookupSampler,
            textureCoords,
            0,
            mipLevel
        ).r;

        let reflectanceForSpecularColorOne = textureSampleLevel(
            specularGGXReflectanceLookupTexture,
            specularGGXReflectanceLookupSampler,
            textureCoords,
            1,
            mipLevel
        ).r;

        let reflectance = (1.0 - specularColor) * reflectanceForSpecularColorZero + specularColor * reflectanceForSpecularColorOne;

        ambientColor = reflectance * ambientRadiance;
    } else {
        ambientColor = vec3<f32>(0.0, 0.0, 0.0);
    }

    return ambientColor;
}

fn getBaseAmbientColor() -> vec3<f32> {
    return vec3<f32>(0.0, 0.0, 0.0);
}

// ***** Omnidirectional lights *****

fn applyCubemapFaceProjectionToPosition(
    position: vec3<f32>,
) -> vec4<f32> {
    // It is important not to perform perspective division manually
    // here, because the homogeneous vector should be interpolated
    // first.
    return vec4<f32>(
        position.xy,
        // This component does not matter, as we compute the proper
        // depth in the fragment shader
        position.z,
        position.z,
    );
}

fn computeShadowMapFragmentDepthOmniLight(
    nearDistance: f32,
    inverseDistanceSpan: f32,
    cubemapSpaceFragmentPosition: vec3<f32>,
) -> f32 {
    // Compute distance between fragment and light and scale to [0, 1] range
    return (length(cubemapSpaceFragmentPosition) - nearDistance) * inverseDistanceSpan;
}

struct LightQuantitiesOmniLight {
    lightDirection: vec3<f32>,
    lightDirectionDotNormalVector: f32,
    attenuatedLightRadiance: vec3<f32>,
    lightSpaceFragmentDisplacement: vec3<f32>,
    normalizedDistance: f32,
}

fn computeLightQuantitiesOmniLight(
    lightPosition: vec3<f32>,
    lightRadiance: vec3<f32>,
    cameraToLightSpaceRotationQuaternion: vec4<f32>,
    nearDistance: f32,
    inverseDistanceSpan: f32,
    fragmentPosition: vec3<f32>,
    fragmentNormal: vec3<f32>,
) -> LightQuantitiesOmniLight {
    var output: LightQuantitiesOmniLight;

    let lightDisplacement = lightPosition - fragmentPosition;
    let inverseSquaredDistance = 1.0 / dot(lightDisplacement, lightDisplacement);
    output.lightDirection = lightDisplacement * sqrt(inverseSquaredDistance);
    output.lightDirectionDotNormalVector = dot(output.lightDirection, fragmentNormal);

    output.attenuatedLightRadiance = lightRadiance * inverseSquaredDistance;

    // Add an offset to the fragment position along the fragment
    // normal to avoid shadow acne. The offset increases as the
    // light becomes less perpendicular to the surface.
    let offsetFragmentDisplacement = -lightDisplacement + fragmentNormal * clamp(1.0 - output.lightDirectionDotNormalVector, 2e-2, 1.0) * 5e-3 / inverseDistanceSpan;

    output.lightSpaceFragmentDisplacement = rotateVectorWithQuaternion(cameraToLightSpaceRotationQuaternion, offsetFragmentDisplacement);
    output.normalizedDistance = (length(output.lightSpaceFragmentDisplacement) - nearDistance) * inverseDistanceSpan;

    return output;
}

fn generateSampleDisplacementOmniLight(
    displacement: vec3<f32>,
    displacementNormalDirection: vec3<f32>,
    displacementBinormalDirection: vec3<f32>,
    sampleOnPerpendicularDisk: vec2<f32>,
) -> vec3<f32> {
    return displacement + sampleOnPerpendicularDisk.x * displacementNormalDirection + sampleOnPerpendicularDisk.y * displacementBinormalDirection;
}

fn computeShadowPenumbraExtentOmniLight(
    shadowMapTexture: texture_depth_cube,
    pointSampler: sampler,
    emissionRadius: f32,
    vogelDiskBaseAngle: f32,
    displacement: vec3<f32>,
    displacementNormalDirection: vec3<f32>,
    displacementBinormalDirection: vec3<f32>,
    referenceDepth: f32,
) -> f32 {
    let sampleDiskRadius: f32 = 0.4;
    let sampleCount: u32 = 8u;

    let inverseSqrtSampleCount = inverseSqrt(f32(sampleCount));

    var averageOccludingDepth: f32 = 0.0;
    var occludingDepthCount: f32 = 0.0;

    for (var sampleIdx: u32 = 0u; sampleIdx < sampleCount; sampleIdx++) {
        let sampleOnPerpendicularDisk = sampleDiskRadius * generateVogelDiskSampleCoords(vogelDiskBaseAngle, inverseSqrtSampleCount, sampleIdx);
        let sampleDisplacement = generateSampleDisplacementOmniLight(displacement, displacementNormalDirection, displacementBinormalDirection, sampleOnPerpendicularDisk);

        let sampledDepth = textureSample(shadowMapTexture, pointSampler, sampleDisplacement);

        if sampledDepth < referenceDepth {
            averageOccludingDepth += sampledDepth;
            occludingDepthCount += 1.0;
        }
    }

    let minPenumbraExtent = 0.01;

    if occludingDepthCount > 0.0 {
        averageOccludingDepth /= occludingDepthCount;
        return max(minPenumbraExtent, emissionRadius * (referenceDepth - averageOccludingDepth) / averageOccludingDepth);
    } else {
        return -1.0;
    }
}

fn computeVogelDiskComparisonSampleAverageOmniLight(
    shadowMapTexture: texture_depth_cube,
    comparisonSampler: sampler_comparison,
    vogelDiskBaseAngle: f32,
    sampleDiskRadius: f32,
    displacement: vec3<f32>,
    displacementNormalDirection: vec3<f32>,
    displacementBinormalDirection: vec3<f32>,
    referenceDepth: f32,
) -> f32 {
    let sample_density = 800.0;

    let sampleCount = u32(clamp(sampleDiskRadius * sample_density, 3.0, 64.0));

    let invSampleCount = 1.0 / f32(sampleCount);
    let inverseSqrtSampleCount = sqrt(invSampleCount);

    var sampleAverage: f32 = 0.0;

    for (var sampleIdx: u32 = 0u; sampleIdx < sampleCount; sampleIdx++) {
        let sampleOnPerpendicularDisk = sampleDiskRadius * generateVogelDiskSampleCoords(vogelDiskBaseAngle, inverseSqrtSampleCount, sampleIdx);
        let sampleDisplacement = generateSampleDisplacementOmniLight(displacement, displacementNormalDirection, displacementBinormalDirection, sampleOnPerpendicularDisk);

        sampleAverage += textureSampleCompare(shadowMapTexture, comparisonSampler, sampleDisplacement, referenceDepth);
    }

    sampleAverage *= invSampleCount;

    return sampleAverage;
}

fn computePCSSLightAccessFactorOmniLight(
    shadowMapTexture: texture_depth_cube,
    pointSampler: sampler,
    comparisonSampler: sampler_comparison,
    emissionRadius: f32,
    cameraFramebufferPosition: vec4<f32>,
    lightSpaceFragmentDisplacement: vec3<f32>,
    referenceDepth: f32,
) -> f32 {
    let vogelDiskBaseAngle = computeVogelDiskBaseAngle(cameraFramebufferPosition);

    let displacementNormalDirection = normalize(findPerpendicularVector(lightSpaceFragmentDisplacement));
    let displacementBinormalDirection = normalize(cross(lightSpaceFragmentDisplacement, displacementNormalDirection));

    let shadowPenumbraExtent = computeShadowPenumbraExtentOmniLight(
        shadowMapTexture,
        pointSampler,
        emissionRadius,
        vogelDiskBaseAngle,
        lightSpaceFragmentDisplacement,
        displacementNormalDirection,
        displacementBinormalDirection,
        referenceDepth,
    );

    if shadowPenumbraExtent < 0.0 {
        return 1.0;
    }

    return computeVogelDiskComparisonSampleAverageOmniLight(
        shadowMapTexture,
        comparisonSampler,
        vogelDiskBaseAngle,
        shadowPenumbraExtent,
        lightSpaceFragmentDisplacement,
        displacementNormalDirection,
        displacementBinormalDirection,
        referenceDepth,
    );
}

fn findPerpendicularVector(vector: vec3<f32>) -> vec3<f32> {
    let shifted_signs = sign(vector) + 0.5;
    let sign_xz = sign(shifted_signs.x * shifted_signs.z);
    let sign_yz = sign(shifted_signs.y * shifted_signs.z);
    return vec3<f32>(sign_xz * vector.z, sign_yz * vector.z, -sign_xz * vector.x - sign_yz * vector.y);
}

// ***** Unidirectional lights *****

fn applyOrthographicProjectionToPosition(
    orthographicTranslation: vec3<f32>,
    orthographicScaling: vec3<f32>,
    position: vec3<f32>
) -> vec3<f32> {
    return (position + orthographicTranslation) * orthographicScaling;
}

fn applyNormalBiasUniLight(
    lightSpacePosition: vec3<f32>,
    lightSpaceNormalVector: vec3<f32>
) -> vec3<f32> {
    let lightDirectionDotNormalVector = -lightSpaceNormalVector.z;
    return lightSpacePosition + lightSpaceNormalVector * clamp(1.0 - lightDirectionDotNormalVector, 0.0, 1.0) * 1e-1;
}

fn computeUniLightClipSpacePosition(
    orthographicTranslation: vec3<f32>,
    orthographicScaling: vec3<f32>,
    lightSpacePosition: vec3<f32>,
    lightSpaceNormalVector: vec3<f32>,
) -> vec3<f32> {
    let biasedLightSpacePosition = applyNormalBiasUniLight(lightSpacePosition, lightSpaceNormalVector);
    return applyOrthographicProjectionToPosition(orthographicTranslation, orthographicScaling, biasedLightSpacePosition);
}

fn determineCascadeIdxMax1(partitionDepths: vec4<f32>, cameraFramebufferPosition: vec4<f32>) -> i32 {
    return 0;
}
 
fn determineCascadeIdxMax2(partitionDepths: vec4<f32>, cameraFramebufferPosition: vec4<f32>) -> i32 {
    var cascadeIdx: i32;
    let depth = cameraFramebufferPosition.z;
    if depth < partitionDepths.x {
        cascadeIdx = 0;
    } else {
        cascadeIdx = 1;
    }
    return cascadeIdx;
}

fn determineCascadeIdxMax3(partitionDepths: vec4<f32>, cameraFramebufferPosition: vec4<f32>) -> i32 {
    var cascadeIdx: i32;
    let depth = cameraFramebufferPosition.z;
    if depth < partitionDepths.x {
        cascadeIdx = 0;
    } else if depth < partitionDepths.y {
        cascadeIdx = 1;
    } else {
        cascadeIdx = 2;
    }
    return cascadeIdx;
}

fn determineCascadeIdxMax4(partitionDepths: vec4<f32>, cameraFramebufferPosition: vec4<f32>) -> i32 {
    var cascadeIdx: i32;
    let depth = cameraFramebufferPosition.z;
    if depth < partitionDepths.x {
        cascadeIdx = 0;
    } else if depth < partitionDepths.y {
        cascadeIdx = 1;
    } else if depth < partitionDepths.z {
        cascadeIdx = 2;
    } else {
        cascadeIdx = 3;
    }
    return cascadeIdx;
}

fn determineCascadeIdxMax5(partitionDepths: vec4<f32>, cameraFramebufferPosition: vec4<f32>) -> i32 {
    var cascadeIdx: i32;
    let depth = cameraFramebufferPosition.z;
    if depth < partitionDepths.x {
        cascadeIdx = 0;
    } else if depth < partitionDepths.y {
        cascadeIdx = 1;
    } else if depth < partitionDepths.z {
        cascadeIdx = 2;
    } else if depth < partitionDepths.w {
        cascadeIdx = 3;
    } else {
        cascadeIdx = 4;
    }
    return cascadeIdx;
}

fn computeShadowPenumbraExtentUniLight(
    shadowMapTexture: texture_depth_2d_array,
    pointSampler: sampler,
    array_index: i32,
    tanAngularRadius: f32,
    vogelDiskBaseAngle: f32,
    worldSpaceToLightClipSpaceXYScale: f32,
    worldSpaceToLightClipSpaceZScale: f32,
    centerTextureCoords: vec2<f32>,
    referenceDepth: f32,
) -> f32 {
    let diskRadius: f32 = 0.4 * worldSpaceToLightClipSpaceXYScale;
    let sampleCount: u32 = 8u;

    let inverseSqrtSampleCount = inverseSqrt(f32(sampleCount));

    var averageOccludingDepth: f32 = 0.0;
    var occludingDepthCount: f32 = 0.0;

    for (var sampleIdx: u32 = 0u; sampleIdx < sampleCount; sampleIdx++) {
        let sampleTextureCoords = centerTextureCoords + diskRadius * generateVogelDiskSampleCoords(vogelDiskBaseAngle, inverseSqrtSampleCount, sampleIdx);
        let sampledDepth = textureSample(shadowMapTexture, pointSampler, sampleTextureCoords, array_index);

        if sampledDepth < referenceDepth {
            averageOccludingDepth += sampledDepth;
            occludingDepthCount += 1.0;
        }
    }

    let minPenumbraExtent = 0.01;

    if occludingDepthCount > 0.0 {
        averageOccludingDepth /= occludingDepthCount;
        return max(minPenumbraExtent, tanAngularRadius * (referenceDepth - averageOccludingDepth) / worldSpaceToLightClipSpaceZScale);
    } else {
        return -1.0;
    }
}

fn computeVogelDiskComparisonSampleAverageUniLight(
    shadowMapTexture: texture_depth_2d_array,
    comparisonSampler: sampler_comparison,
    array_index: i32,
    vogelDiskBaseAngle: f32,
    worldSpaceToLightClipSpaceXYScale: f32,
    worldSpaceDiskRadius: f32,
    centerTextureCoords: vec2<f32>,
    referenceDepth: f32,
) -> f32 {
    let sample_density = 800.0;

    let sampleCount = u32(clamp(worldSpaceDiskRadius * sample_density, 3.0, 64.0));

    let diskRadius = worldSpaceDiskRadius * worldSpaceToLightClipSpaceXYScale;

    let invSampleCount = 1.0 / f32(sampleCount);
    let inverseSqrtSampleCount = sqrt(invSampleCount);

    var sampleAverage: f32 = 0.0;

    for (var sampleIdx: u32 = 0u; sampleIdx < sampleCount; sampleIdx++) {
        let sampleTextureCoords = centerTextureCoords + diskRadius * generateVogelDiskSampleCoords(vogelDiskBaseAngle, inverseSqrtSampleCount, sampleIdx);
        sampleAverage += textureSampleCompare(shadowMapTexture, comparisonSampler, sampleTextureCoords, array_index, referenceDepth);
    }

    sampleAverage *= invSampleCount;

    return sampleAverage;
}

fn computePCSSLightAccessFactorUniLight(
    shadowMapTexture: texture_depth_2d_array,
    pointSampler: sampler,
    comparisonSampler: sampler_comparison,
    array_index: i32,
    tanAngularRadius: f32,
    worldSpaceToLightClipSpaceXYScale: f32,
    worldSpaceToLightClipSpaceZScale: f32,
    cameraFramebufferPosition: vec4<f32>,
    centerTextureCoords: vec2<f32>,
    referenceDepth: f32,
) -> f32 {
    let vogelDiskBaseAngle = computeVogelDiskBaseAngle(cameraFramebufferPosition);

    let shadowPenumbraExtent = computeShadowPenumbraExtentUniLight(
        shadowMapTexture,
        pointSampler,
        array_index,
        tanAngularRadius,
        vogelDiskBaseAngle,
        worldSpaceToLightClipSpaceXYScale,
        worldSpaceToLightClipSpaceZScale,
        centerTextureCoords,
        referenceDepth,
    );

    if shadowPenumbraExtent < 0.0 {
        return 1.0;
    }

    return computeVogelDiskComparisonSampleAverageUniLight(
        shadowMapTexture,
        comparisonSampler,
        array_index,
        vogelDiskBaseAngle,
        worldSpaceToLightClipSpaceXYScale,
        shadowPenumbraExtent,
        centerTextureCoords,
        referenceDepth,
    );
}

// ***** Common shadow mapping utilities *****

// Returns a random number between 0 and 1 based on the pixel coordinates
fn generateInterleavedGradientNoiseFactor(cameraFramebufferPosition: vec4<f32>) -> f32 {
    let magic = vec3<f32>(0.06711056, 0.00583715, 52.9829189);
    return fract(magic.z * fract(dot(magic.xy, cameraFramebufferPosition.xy)));
}

fn generateVogelDiskSampleCoords(baseAngle: f32, inverseSqrtSampleCount: f32, sampleIdx: u32) -> vec2<f32> {
    let goldenAngle: f32 = 2.4;
    let radius = sqrt(f32(sampleIdx) + 0.5) * inverseSqrtSampleCount;
    let angle = baseAngle + goldenAngle * f32(sampleIdx);
    return vec2<f32>(radius * cos(angle), radius * sin(angle));
}

fn computeVogelDiskBaseAngle(cameraFramebufferPosition: vec4<f32>) -> f32 {
    // Multiply with 2 * pi to get random angle
    return 6.283185307 * generateInterleavedGradientNoiseFactor(cameraFramebufferPosition);
}

