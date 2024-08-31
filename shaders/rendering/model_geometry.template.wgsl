struct PushConstants {
    // Split up inverseWindowDimensions to avoid padding
    inverseWindowWidth: f32,
    inverseWindowHeight: f32,
    frameCounter: u32,
    exposure: f32,
}

struct ProjectionUniform {
    projection: mat4x4f,
    frustumFarPlaneCorners: array<vec4f, 4>,
    inverseFarPlaneZ: vec4f,
    jitterOffsets: array<vec4f, {{jitter_count}}>,
}

struct ModelViewTransform {
    @location({{model_view_transform_rotation_location}}) rotationQuaternion: vec4f,
    @location({{model_view_transform_translation_location}}) translationAndScaling: vec4f,
}

struct PreviousModelViewTransform {
    @location({{previous_model_view_transform_rotation_location}}) rotationQuaternion: vec4f,
    @location({{previous_model_view_transform_translation_location}}) translationAndScaling: vec4f,
}

struct MaterialProperties {
    @location({{specular_reflectance_location}}) specularReflectance: f32,
    @location({{roughness_location}}) roughness: f32,
    @location({{metalness_location}}) metalness: f32,
    @location({{emissive_luminance_location}}) emissiveLuminance: f32,
#if (has_color_value)
    @location({{color_location}}) color: vec3f,
#endif
#if (uses_parallax_mapping)
    @location({{parallax_displacement_scale_location}}) parallaxDisplacementScale: f32,
    @location({{parallax_uv_per_distance_location}}) parallaxUVPerDistance: vec2f,
#endif
}

struct VertexInput {
    @location({{position_location}}) modelSpacePosition: vec3f,
#if (has_normal_vector)
    @location({{normal_vector_location}}) modelSpaceNormalVector: vec3f,
#endif
#if (has_texture_coords)
    @location({{texture_coords_location}}) textureCoords: vec2f,
#endif
#if (has_tangent_space_quaternion)
    @location({{tangent_space_quaternion_location}}) tangentToModelSpaceRotationQuaternion: vec4f,
#endif
}

struct FragmentInput {
    @builtin(position) projectedPosition: vec4f,
    @location(0) previousClipSpacePosition: vec4f,
    @location(1) cameraSpacePosition: vec3f,
#if (has_normal_vector)
    @location(2) cameraSpaceNormalVector: vec3f,
#endif
#if (has_texture_coords)
    @location(3) textureCoords: vec2f,
#endif
#if (has_tangent_space_quaternion)
    @location(4) tangentToCameraSpaceRotationQuaternion: vec4f,
#endif
    @location(5) specularReflectance: f32,
    @location(6) roughness: f32,
    @location(7) metalness: f32,
    @location(8) emissiveLuminance: f32,
#if (has_color_value)
    @location(9) color: vec3f,
#endif
#if (uses_parallax_mapping)
    @location(10) parallaxDisplacementScale: f32,
    @location(11) parallaxUVPerDistance: vec2f,
#endif
}

struct FragmentOutput {
    @location(0) linearDepth: f32,
    @location(1) normalVector: vec4f,
    @location(2) motionVector: vec2f,
    @location(3) materialColor: vec4f,
    @location(4) materialProperties: vec4f,
}

const JITTER_COUNT: u32 = {{jitter_count}};

var<push_constant> pushConstants: PushConstants;

@group({{projection_uniform_group}}) @binding({{projection_uniform_binding}})
var<uniform> projectionUniform: ProjectionUniform;

#if (uses_parallax_mapping)
@group({{material_texture_group}}) @binding({{height_map_texture_binding}})
var heightMapTexture: texture_2d<f32>;
@group({{material_texture_group}}) @binding({{height_map_sampler_binding}})
var heightMapSampler: sampler;
#endif

#if (uses_normal_mapping)
@group({{material_texture_group}}) @binding({{normal_map_texture_binding}})
var normalMapTexture: texture_2d<f32>;
@group({{material_texture_group}}) @binding({{normal_map_sampler_binding}})
var normalMapSampler: sampler;
#endif

#if (has_color_texture)
@group({{material_texture_group}}) @binding({{material_color_texture_binding}})
var materialColorTexture: texture_2d<f32>;
@group({{material_texture_group}}) @binding({{material_color_sampler_binding}})
var materialColorSampler: sampler;
#endif

#if (has_specular_reflectance_texture)
@group({{material_texture_group}}) @binding({{specular_reflectance_texture_binding}})
var specularReflectanceTexture: texture_2d<f32>;
@group({{material_texture_group}}) @binding({{specular_reflectance_sampler_binding}})
var specularReflectanceSampler: sampler;
#endif

#if (has_roughness_texture)
@group({{material_texture_group}}) @binding({{roughness_texture_binding}})
var roughnessTexture: texture_2d<f32>;
@group({{material_texture_group}}) @binding({{roughness_sampler_binding}})
var roughnessSampler: sampler;
#endif

#if (has_metalness_texture)
@group({{material_texture_group}}) @binding({{metalness_texture_binding}})
var metalnessTexture: texture_2d<f32>;
@group({{material_texture_group}}) @binding({{metalness_sampler_binding}})
var metalnessSampler: sampler;
#endif

#if (has_emissive_luminance_texture)
@group({{material_texture_group}}) @binding({{emissive_luminance_texture_binding}})
var emissiveLuminanceTexture: texture_2d<f32>;
@group({{material_texture_group}}) @binding({{emissive_luminance_sampler_binding}})
var emissiveLuminanceSampler: sampler;
#endif

fn transformPosition(
    rotationQuaternion: vec4f,
    translation: vec3f,
    scaling: f32,
    position: vec3f
) -> vec3f {
    return rotateVectorWithQuaternion(rotationQuaternion, scaling * position) + translation;
}

fn rotateVectorWithQuaternion(quaternion: vec4f, vector: vec3f) -> vec3f {
    let tmp = 2.0 * cross(quaternion.xyz, vector);
    return vector + quaternion.w * tmp + cross(quaternion.xyz, tmp);
}

fn applyRotationToTangentSpaceQuaternion(
    rotationQuaternion: vec4f,
    tangentToParentSpaceRotationQuaternion: vec4f,
) -> vec4f {
    let q1 = rotationQuaternion;
    let q2 = tangentToParentSpaceRotationQuaternion;
    var rotated = normalize(vec4f(q1.w * q2.xyz + q2.w * q1.xyz + cross(q1.xyz, q2.xyz), q1.w * q2.w - dot(q1.xyz, q2.xyz)));

    // Preserve encoding of tangent space handedness in real component of
    // tangent space quaternion
    if (rotated.w < 0.0) != (tangentToParentSpaceRotationQuaternion.w < 0.0) {
        rotated = -rotated;
    }

    return rotated;
}

fn obtainProjectionMatrix() -> mat4x4f {
    var matrix = projectionUniform.projection;
    let jitterIndex = pushConstants.frameCounter % JITTER_COUNT;
    let jitterOffsets = projectionUniform.jitterOffsets[jitterIndex];
    matrix[2][0] += jitterOffsets.x * pushConstants.inverseWindowWidth;
    matrix[2][1] += jitterOffsets.y * pushConstants.inverseWindowHeight;
    return matrix;
}

#if (uses_parallax_mapping)
fn computeCameraSpaceViewDirection(cameraSpacePosition: vec3f) -> vec3f {
    return normalize(-cameraSpacePosition);
}

fn computeParallaxMappedTextureCoordinates(
    displacementScale: f32,
    originalTextureCoords: vec2f,
    tangentToCameraSpaceRotationQuaternion: vec4f,
    cameraSpaceViewDirection: vec3f,
) -> vec2f {
    let hardMaxMipLevel = 7.0;
    let softMaxMipLevel = 6.0;

    let tangentSpaceViewDirection: vec3f = transformVectorToTangentSpace(tangentToCameraSpaceRotationQuaternion, cameraSpaceViewDirection);

    var parallaxMappedTextureCoords: vec2f = originalTextureCoords;

    if tangentSpaceViewDirection.z > 0.0 {
        let mipLevel = computeParallaxMappingLevelOfDetail(textureDimensions(heightMapTexture), originalTextureCoords);

        // Skip parallax mapping if the level of detail is low enough
        if mipLevel <= hardMaxMipLevel {
            let maxLayerCount = mix(64.0, 8.0, max(0.0, tangentSpaceViewDirection.z));
            let layerDepth = displacementScale / maxLayerCount;

            let textureCoordOffsetVector = tangentSpaceViewDirection.xy * (layerDepth / tangentSpaceViewDirection.z);

            var currentLayerCount = 0.0;
            var currentDepth = 0.0;
            var prevTextureCoords = originalTextureCoords;
            var currentTextureCoords = originalTextureCoords;

            let sampledHeight = textureSampleLevel(heightMapTexture, heightMapSampler, currentTextureCoords, mipLevel).r;
            var currentSampledDepth = (1.0 - sampledHeight) * displacementScale;
            var prevSampledDepth = currentSampledDepth;

            while currentSampledDepth > currentDepth && currentLayerCount < maxLayerCount {
                prevTextureCoords = currentTextureCoords;
                prevSampledDepth = currentSampledDepth;

                currentTextureCoords -= textureCoordOffsetVector;
                currentDepth += layerDepth;

                let sampledHeight = textureSampleLevel(heightMapTexture, heightMapSampler, currentTextureCoords, mipLevel).r;
                currentSampledDepth = (1.0 - sampledHeight) * displacementScale;

                currentLayerCount += 1.0;
            }

            let currentDepthDiff = currentSampledDepth - currentDepth;
            let prevDepthDiff = prevSampledDepth - (currentDepth - layerDepth);
            let differenceInDepthDiff = currentDepthDiff - prevDepthDiff;

            var interpWeightForZeroDepthDiff = 0.0;
            if abs(differenceInDepthDiff) > 1e-6 {
                interpWeightForZeroDepthDiff = currentDepthDiff / differenceInDepthDiff;
            }

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

fn computeParallaxMappingLevelOfDetail(textureDims: vec2u, textureCoords: vec2f) -> f32 {
    let texelPosition = textureCoords * vec2f(textureDims);
    let duvdx = dpdx(texelPosition);
    let duvdy = dpdy(texelPosition);
    let duv = duvdx * duvdx + duvdy * duvdy;
    let maxduv = max(max(duv.x, duv.y), 1e-6);
    return max(0.0, 0.5 * log2(maxduv));
}

fn obtainTangentSpaceNormalFromHeightMap(
    heightScale: f32,
    uvPerDistance: vec2f,
    textureCoords: vec2f,
) -> vec3f {
    let textureDims = textureDimensions(heightMapTexture);

    let offsetU = vec2f(1.0 / f32(textureDims.x), 0.0);
    let offsetV = vec2f(0.0, 1.0 / f32(textureDims.y));

    let heightDownU = textureSample(heightMapTexture, heightMapSampler, textureCoords - offsetU).r;
    let heightUpU = textureSample(heightMapTexture, heightMapSampler, textureCoords + offsetU).r;
    let heightDownV = textureSample(heightMapTexture, heightMapSampler, textureCoords - offsetV).r;
    let heightUpV = textureSample(heightMapTexture, heightMapSampler, textureCoords + offsetV).r;

    return -normalize(vec3f(
        (heightUpU - heightDownU) * heightScale * 0.5 * f32(textureDims.x) * uvPerDistance.x,
        (heightUpV - heightDownV) * heightScale * 0.5 * f32(textureDims.y) * uvPerDistance.y,
        -1.0,
    ));
}

fn transformVectorToTangentSpace(
    tangentToParentSpaceRotationQuaternion: vec4f,
    parentSpaceVector: vec3f,
) -> vec3f {
    var tangentSpaceVector = rotateVectorWithInverseOfQuaternion(tangentToParentSpaceRotationQuaternion, parentSpaceVector);

    // If the real component is negative, tangent space is really left-handed
    // and we have to flip the y (bitangent) component of the tangent space
    // vector after applying the rotation
    if tangentToParentSpaceRotationQuaternion.w < 0.0 {
        tangentSpaceVector.y = -tangentSpaceVector.y;
    }

    return tangentSpaceVector;
}

fn rotateVectorWithInverseOfQuaternion(quaternion: vec4f, vector: vec3f) -> vec3f {
    let tmp = 2.0 * cross(quaternion.xyz, vector);
    return vector - quaternion.w * tmp + cross(quaternion.xyz, tmp);
}
#endif // uses_parallax_mapping

#if (uses_normal_mapping)
// From [0, 1] to [-1, 1]
fn convertNormalColorToNormalVector(color: vec3f) -> vec3f {
    // May require normalization depending on filtering
    return 2.0 * (color - 0.5);
}
#endif

fn transformVectorFromTangentSpace(
    tangentToParentSpaceRotationQuaternion: vec4f,
    tangentSpaceVector: vec3f,
) -> vec3f {
    var correctedTangentSpaceVector = tangentSpaceVector;

    // If the real component is negative, tangent space is really left-handed
    // and we have to flip the y (bitangent) component of the tangent space
    // vector before applying the rotation
    if tangentToParentSpaceRotationQuaternion.w < 0.0 {
        correctedTangentSpaceVector.y = -correctedTangentSpaceVector.y;
    }

    return rotateVectorWithQuaternion(tangentToParentSpaceRotationQuaternion, correctedTangentSpaceVector);
}

fn computeGGXRoughnessFromPerceptuallyLinearRoughness(linearRoughness: f32) -> f32 {
    return linearRoughness * linearRoughness;
}

fn convertFramebufferPositionToScreenTextureCoords(framebufferPosition: vec4f) -> vec2<f32> {
    return framebufferPosition.xy * vec2f(pushConstants.inverseWindowWidth, pushConstants.inverseWindowHeight);
}

fn computeMotionVector(
    screenTextureCoords: vec2f,
    previousClipSpacePosition: vec4f,
) -> vec2f {
    if (previousClipSpacePosition.w < 1e-6) {
        // The previous position is behind the camera
        return vec2f(1.0, 1.0);
    }
    let previousNDCXYPosition = previousClipSpacePosition.xy / previousClipSpacePosition.w;
    let previousScreenCoords = vec2f(0.5 * (1.0 + previousNDCXYPosition.x), 0.5 * (1.0 - previousNDCXYPosition.y));
    return screenTextureCoords - previousScreenCoords;
}

// From [-1, 1] to [0, 1]
fn convertNormalVectorToNormalColor(normalVector: vec3f) -> vec3f {
    return 0.5 * (normalVector + 1.0);
}

@vertex
fn mainVS(
    vertex: VertexInput,
    modelViewTransform: ModelViewTransform,
    previousModelViewTransform: PreviousModelViewTransform,
    material: MaterialProperties,
) -> FragmentInput {
    var output: FragmentInput;

    let projectionMatrix = obtainProjectionMatrix();

    let cameraSpacePosition = transformPosition(
        modelViewTransform.rotationQuaternion,
        modelViewTransform.translationAndScaling.xyz,
        modelViewTransform.translationAndScaling.w,
        vertex.modelSpacePosition,
    );
    output.projectedPosition = projectionMatrix * vec4f(cameraSpacePosition, 1.0);
    output.cameraSpacePosition = cameraSpacePosition;

    let previousCameraSpacePosition = transformPosition(
        previousModelViewTransform.rotationQuaternion,
        previousModelViewTransform.translationAndScaling.xyz,
        previousModelViewTransform.translationAndScaling.w,
        vertex.modelSpacePosition,
    );
    output.previousClipSpacePosition = projectionMatrix * vec4f(previousCameraSpacePosition, 1.0);

#if (has_normal_vector)
    output.cameraSpaceNormalVector = rotateVectorWithQuaternion(
        modelViewTransform.rotationQuaternion,
        vertex.modelSpaceNormalVector,
    );
#endif

#if (has_texture_coords)
    output.textureCoords = vertex.textureCoords;
#endif

#if (has_tangent_space_quaternion)
    output.tangentToCameraSpaceRotationQuaternion = applyRotationToTangentSpaceQuaternion(
        modelViewTransform.rotationQuaternion,
        vertex.tangentToModelSpaceRotationQuaternion,
    );
#endif

    output.specularReflectance = material.specularReflectance;
    output.roughness = material.roughness;
    output.metalness = material.metalness;
    output.emissiveLuminance = material.emissiveLuminance;
#if (has_color_value)
    output.color = material.color;
#endif
#if (uses_parallax_mapping)
    output.parallaxDisplacementScale = material.parallaxDisplacementScale;
    output.parallaxUVPerDistance = material.parallaxUVPerDistance;
#endif

    return output;
}

@fragment
fn mainFS(fragment: FragmentInput) -> FragmentOutput {
    var output: FragmentOutput;

    output.linearDepth = projectionUniform.inverseFarPlaneZ.x * fragment.cameraSpacePosition.z;

#if (has_texture_coords)
    var textureCoords = fragment.textureCoords;
#endif

    var cameraSpaceNormalVector: vec3f;
#if (uses_parallax_mapping)
    let cameraSpaceViewDirection = computeCameraSpaceViewDirection(fragment.cameraSpacePosition);
    let tangentToCameraSpaceRotationQuaternion = normalize(fragment.tangentToCameraSpaceRotationQuaternion);
    textureCoords = computeParallaxMappedTextureCoordinates(
        fragment.parallaxDisplacementScale,
        textureCoords,
        tangentToCameraSpaceRotationQuaternion,
        cameraSpaceViewDirection,
    );
    let tangentSpaceNormalVector = obtainTangentSpaceNormalFromHeightMap(
        fragment.parallaxDisplacementScale,
        fragment.parallaxUVPerDistance,
        textureCoords,
    );
    cameraSpaceNormalVector = transformVectorFromTangentSpace(
        tangentToCameraSpaceRotationQuaternion,
        tangentSpaceNormalVector,
    );
#elseif (uses_normal_mapping)
    let normalColor = textureSample(normalMapTexture, normalMapSampler, textureCoords).rgb;
    let tangentSpaceNormalVector = convertNormalColorToNormalVector(normalColor);
    let tangentToCameraSpaceRotationQuaternion = normalize(fragment.tangentToCameraSpaceRotationQuaternion);
    cameraSpaceNormalVector = transformVectorFromTangentSpace(
        tangentToCameraSpaceRotationQuaternion,
        tangentSpaceNormalVector,
    );
#else // (has_normal_vector)
    cameraSpaceNormalVector = fragment.cameraSpaceNormalVector;
#endif

    output.normalVector = vec4f(convertNormalVectorToNormalColor(cameraSpaceNormalVector), 1.0);

    let screenTextureCoords = convertFramebufferPositionToScreenTextureCoords(
        fragment.projectedPosition,
    );
    output.motionVector = computeMotionVector(screenTextureCoords, fragment.previousClipSpacePosition);

    var materialColor: vec3f;
#if (has_color_texture)
    materialColor = textureSample(materialColorTexture, materialColorSampler, textureCoords).rgb;
#else // (has_color_value)
    materialColor = fragment.color;
#endif
    output.materialColor = vec4f(materialColor, 1.0);

    var specularReflectance: f32;
#if (has_specular_reflectance_texture)
    specularReflectance = textureSample(specularReflectanceTexture, specularReflectanceSampler, textureCoords).r;
    specularReflectance *= fragment.specularReflectance; // Apply scale factor
#else
    specularReflectance = fragment.specularReflectance;
#endif

    var roughness: f32;
#if (has_roughness_texture)
    roughness = textureSample(roughnessTexture, roughnessSampler, textureCoords).r;
    roughness *= fragment.roughness; // Apply scale factor
    roughness = computeGGXRoughnessFromPerceptuallyLinearRoughness(roughness);
#else
    roughness = computeGGXRoughnessFromPerceptuallyLinearRoughness(fragment.roughness);
#endif

    var metalness: f32;
#if (has_metalness_texture)
    metalness = textureSample(metalnessTexture, metalnessSampler, textureCoords).r;
    metalness *= fragment.metalness; // Apply scale factor
#else
    metalness = fragment.metalness;
#endif

    var emissiveLuminance: f32;
#if (has_emissive_luminance_texture)
    emissiveLuminance = textureSample(emissiveLuminanceTexture, emissiveLuminanceSampler, textureCoords).r;
    emissiveLuminance *= fragment.emissiveLuminance; // Apply scale factor
#else
    emissiveLuminance = fragment.emissiveLuminance;
#endif
    let preExposedEmissiveLuminance = emissiveLuminance * pushConstants.exposure;

    output.materialProperties = vec4f(specularReflectance, roughness, metalness, preExposedEmissiveLuminance);

    return output;
}
