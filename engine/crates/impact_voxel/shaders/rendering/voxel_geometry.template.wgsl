struct PushConstants {
    // Split up inverseWindowDimensions to avoid padding
    inverseWindowWidth: f32,
    inverseWindowHeight: f32,
    frameCounter: u32,
    exposure: f32,
    originOffsetInRootX: f32,
    originOffsetInRootY: f32,
    originOffsetInRootZ: f32,
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

struct VertexInput {
    @location({{index_location}}) vertexIndex: u32,
    @location({{material_indices_location}}) materialIndices: vec4u,
    @location({{material_weights_location}}) materialWeights: vec4u,
}

struct FragmentInput {
    @builtin(position) projectedPosition: vec4f,
    @location(0) previousClipSpacePosition: vec4f,
    @location(1) offsetModelSpacePosition: vec3f,
    @location(2) cameraSpacePosition: vec3f,
    @location(3) modelSpaceNormalVector: vec3f,
    @location(4) @interpolate(flat) modelToCameraSpaceRotationQuaternion: vec4f,
    @location(5) @interpolate(flat) materialIndices: vec4u,
    @location(6) materialWeights: vec4f,
    @location(7) uniformMaterialProperties: vec4f,
}

struct FragmentOutput {
    @location(0) linearDepth: f32,
    @location(1) normalVector: vec4f,
    @location(2) motionVector: vec2f,
    @location(3) materialColor: vec4f,
    @location(4) materialProperties: vec4f,
}

const JITTER_COUNT: u32 = {{jitter_count}};

const TEXTURE_FREQUENCY: f32 = {{texture_frequency}};

const FACE_NORMAL_TRANSITION_START_ALIGNMENT: f32 = 0.8;
const FACE_NORMAL_TRANSITION_END_ALIGNMENT: f32 = 0.6;
const FACE_NORMAL_TRANSITION_NORM: f32 = 1.0 / (FACE_NORMAL_TRANSITION_START_ALIGNMENT - FACE_NORMAL_TRANSITION_END_ALIGNMENT);

var<push_constant> pushConstants: PushConstants;

@group({{projection_uniform_group}}) @binding({{projection_uniform_binding}})
var<uniform> projectionUniform: ProjectionUniform;

@group({{material_group}}) @binding({{fixed_material_uniform_binding}})
var<uniform> fixedMaterialProperties: array<vec4f, {{voxel_type_count}}>;

@group({{material_group}}) @binding({{color_texture_array_binding}})
var materialColorTextures: texture_2d_array<f32>;

@group({{material_group}}) @binding({{roughness_texture_array_binding}})
var materialRoughnessTextures: texture_2d_array<f32>;

@group({{material_group}}) @binding({{normal_texture_array_binding}})
var materialNormalTextures: texture_2d_array<f32>;

@group({{material_group}}) @binding({{sampler_binding}})
var materialSampler: sampler;

// We represent the positions as an array of `f32` components rather than
// `vec3f` because the latter will be assumed aligned to 16 bytes, which is not
// the case for the actual data
@group({{position_and_normal_group}}) @binding({{position_buffer_binding}})
var<storage, read> modelSpaceVertexPositions: array<f32>;

@group({{position_and_normal_group}}) @binding({{normal_buffer_binding}})
var<storage, read> modelSpaceVertexNormalVectors: array<f32>;

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

fn obtainProjectionMatrix() -> mat4x4f {
    var matrix = projectionUniform.projection;
    let jitterIndex = pushConstants.frameCounter % JITTER_COUNT;
    let jitterOffsets = projectionUniform.jitterOffsets[jitterIndex];
    matrix[2][0] += jitterOffsets.x * pushConstants.inverseWindowWidth;
    matrix[2][1] += jitterOffsets.y * pushConstants.inverseWindowHeight;
    return matrix;
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

// From [0, 1] to [-1, 1]
fn convertNormalColorToNormalVector(color: vec3f) -> vec3f {
    // May require normalization depending on filtering
    return 2.0 * (color - 0.5);
}

fn getMaxComponent(vector: vec4f) -> f32 {
    return max(max(vector.x, vector.y), max(vector.z, vector.w));
}

fn triplanarSampleTexture(
    textureArray: texture_2d_array<f32>,
    textureSampler: sampler,
    weights: vec3f,
    coordsX: vec2f,
    coordsY: vec2f,
    coordsZ: vec2f,
    arrayIdx: u32,
) -> vec4f {
    let sampleX = textureSample(textureArray, textureSampler, coordsX, arrayIdx);
    let sampleY = textureSample(textureArray, textureSampler, coordsY, arrayIdx);
    let sampleZ = textureSample(textureArray, textureSampler, coordsZ, arrayIdx);
    return weights.x * sampleX + weights.y * sampleY + weights.z * sampleZ;
}

fn triplanarSampleNormalTexture(
    textureArray: texture_2d_array<f32>,
    textureSampler: sampler,
    modelSpaceNormalVector: vec3f,
    weights: vec3f,
    coordsX: vec2f,
    coordsY: vec2f,
    coordsZ: vec2f,
    arrayIdx: u32,
) -> vec3f {
    var tangentSpaceNormalX = convertNormalColorToNormalVector(textureSample(textureArray, textureSampler, coordsX, arrayIdx).rgb);
    var tangentSpaceNormalY = convertNormalColorToNormalVector(textureSample(textureArray, textureSampler, coordsY, arrayIdx).rgb);
    var tangentSpaceNormalZ = convertNormalColorToNormalVector(textureSample(textureArray, textureSampler, coordsZ, arrayIdx).rgb);

    // To convert the sampled tangent space normals to model space, we will
    // swizzle each of them based on which plane its normal texture was
    // projected into. But first we make sure their orientation will align with
    // the unbumped surface normal using a Whiteout blend (see e.g.
    // https://bgolus.medium.com/normal-mapping-for-a-triplanar-shader-10bf39dca05a).
    tangentSpaceNormalX = vec3f(tangentSpaceNormalX.xy + modelSpaceNormalVector.zy, abs(tangentSpaceNormalX.z) * modelSpaceNormalVector.x);
    tangentSpaceNormalY = vec3f(tangentSpaceNormalY.xy + modelSpaceNormalVector.xz, abs(tangentSpaceNormalY.z) * modelSpaceNormalVector.y);
    tangentSpaceNormalZ = vec3f(tangentSpaceNormalZ.xy + modelSpaceNormalVector.xy, abs(tangentSpaceNormalZ.z) * modelSpaceNormalVector.z);

    let modelSpaceNormalX = tangentSpaceNormalX.zyx;
    let modelSpaceNormalY = tangentSpaceNormalY.xzy;
    let modelSpaceNormalZ = tangentSpaceNormalZ.xyz;

    return normalize(weights.x * modelSpaceNormalX + weights.y * modelSpaceNormalY + weights.z * modelSpaceNormalZ);
}

fn triplanarSampleAndBlendTextures(
    textureArray: texture_2d_array<f32>,
    textureSampler: sampler,
    triplanarWeights: vec3f,
    coordsX: vec2f,
    coordsY: vec2f,
    coordsZ: vec2f,
    materialIndices: vec4u,
    materialWeights: vec4f,
) -> vec4f {
    let sample1 = triplanarSampleTexture(textureArray, textureSampler, triplanarWeights, coordsX, coordsY, coordsZ, materialIndices.x);
    let sample2 = triplanarSampleTexture(textureArray, textureSampler, triplanarWeights, coordsX, coordsY, coordsZ, materialIndices.y);
    let sample3 = triplanarSampleTexture(textureArray, textureSampler, triplanarWeights, coordsX, coordsY, coordsZ, materialIndices.z);
    let sample4 = triplanarSampleTexture(textureArray, textureSampler, triplanarWeights, coordsX, coordsY, coordsZ, materialIndices.w);
    return materialWeights.x * sample1 +
           materialWeights.y * sample2 +
           materialWeights.z * sample3 +
           materialWeights.w * sample4;
}

fn triplanarSampleAndBlendNormalTextures(
    textureArray: texture_2d_array<f32>,
    textureSampler: sampler,
    modelSpaceNormal: vec3f,
    triplanarWeights: vec3f,
    coordsX: vec2f,
    coordsY: vec2f,
    coordsZ: vec2f,
    materialIndices: vec4u,
    materialWeights: vec4f,
) -> vec3f {
    let sample1 = triplanarSampleNormalTexture(textureArray, textureSampler, modelSpaceNormal, triplanarWeights, coordsX, coordsY, coordsZ, materialIndices.x);
    let sample2 = triplanarSampleNormalTexture(textureArray, textureSampler, modelSpaceNormal, triplanarWeights, coordsX, coordsY, coordsZ, materialIndices.y);
    let sample3 = triplanarSampleNormalTexture(textureArray, textureSampler, modelSpaceNormal, triplanarWeights, coordsX, coordsY, coordsZ, materialIndices.z);
    let sample4 = triplanarSampleNormalTexture(textureArray, textureSampler, modelSpaceNormal, triplanarWeights, coordsX, coordsY, coordsZ, materialIndices.w);
    return normalize(
        materialWeights.x * sample1 +
        materialWeights.y * sample2 +
        materialWeights.z * sample3 +
        materialWeights.w * sample4
    );
}

fn computeGGXRoughnessFromPerceptuallyLinearRoughness(linearRoughness: f32) -> f32 {
    return linearRoughness * linearRoughness;
}

fn computeFaceNormal(position: vec3f) -> vec3f {
    let dx = dpdx(position);
    let dy = dpdy(position);
    return normalize(cross(dx, -dy));
}

@vertex
fn mainVS(
    vertex: VertexInput,
    modelViewTransform: ModelViewTransform,
    previousModelViewTransform: PreviousModelViewTransform,
) -> FragmentInput {
    var output: FragmentInput;

    let xCompIdx = 3 * vertex.vertexIndex;
    let yCompIdx = xCompIdx + 1;
    let zCompIdx = xCompIdx + 2;

    let modelSpacePosition = vec3f(
        modelSpaceVertexPositions[xCompIdx],
        modelSpaceVertexPositions[yCompIdx],
        modelSpaceVertexPositions[zCompIdx],
    );
    output.modelSpaceNormalVector = vec3f(
        modelSpaceVertexNormalVectors[xCompIdx],
        modelSpaceVertexNormalVectors[yCompIdx],
        modelSpaceVertexNormalVectors[zCompIdx],
    );

    let projectionMatrix = obtainProjectionMatrix();

    let cameraSpacePosition = transformPosition(
        modelViewTransform.rotationQuaternion,
        modelViewTransform.translationAndScaling.xyz,
        modelViewTransform.translationAndScaling.w,
        modelSpacePosition,
    );
    output.projectedPosition = projectionMatrix * vec4f(cameraSpacePosition, 1.0);
    output.cameraSpacePosition = cameraSpacePosition;

    let previousCameraSpacePosition = transformPosition(
        previousModelViewTransform.rotationQuaternion,
        previousModelViewTransform.translationAndScaling.xyz,
        previousModelViewTransform.translationAndScaling.w,
        modelSpacePosition,
    );
    output.previousClipSpacePosition = projectionMatrix * vec4f(previousCameraSpacePosition, 1.0);

    output.modelToCameraSpaceRotationQuaternion = modelViewTransform.rotationQuaternion;

    output.materialIndices = vertex.materialIndices;
    output.materialWeights = vec4f(vertex.materialWeights);

    // In case the voxel object has been disconnected from another object, we
    // need to use the model space position relative to the origin of the
    // original object. Otherwise, the triplanar texture coordinates would
    // shift along with the origin during splitting, causing the textures to
    // change abruptly
    output.offsetModelSpacePosition = vec3f(
        pushConstants.originOffsetInRootX,
        pushConstants.originOffsetInRootY,
        pushConstants.originOffsetInRootZ,
    ) + modelSpacePosition;

    return output;
}

@fragment
fn mainFS(fragment: FragmentInput) -> FragmentOutput {
    var output: FragmentOutput;

    output.linearDepth = projectionUniform.inverseFarPlaneZ.x * fragment.cameraSpacePosition.z;

    let screenTextureCoords = convertFramebufferPositionToScreenTextureCoords(
        fragment.projectedPosition,
    );
    output.motionVector = computeMotionVector(screenTextureCoords, fragment.previousClipSpacePosition);

    var modelSpaceNormalVector = normalize(fragment.modelSpaceNormalVector);

    let modelSpaceFaceNormalVector = computeFaceNormal(fragment.offsetModelSpacePosition);

    // If the normal vector deviates significantly from the face normal (which
    // can happen if adjacent triangles have very different face normals), we
    // should transition to using the face normal to avoid artifacts.
    let alignment = dot(modelSpaceNormalVector, modelSpaceFaceNormalVector);
    if (alignment < FACE_NORMAL_TRANSITION_START_ALIGNMENT) {
        // Create a smooth transition between the normals depending on how
        // aligned they are
        let weight = (FACE_NORMAL_TRANSITION_START_ALIGNMENT - alignment) * FACE_NORMAL_TRANSITION_NORM;
        modelSpaceNormalVector = normalize(mix(modelSpaceNormalVector, modelSpaceFaceNormalVector, weight));
    }

    var triplanarWeights = abs(modelSpaceNormalVector);
    triplanarWeights *= triplanarWeights * triplanarWeights; // Raise to 3rd power
    triplanarWeights /= triplanarWeights.x + triplanarWeights.y + triplanarWeights.z;

    let triplanarCoordsX = TEXTURE_FREQUENCY * fragment.offsetModelSpacePosition.zy;
    let triplanarCoordsY = TEXTURE_FREQUENCY * fragment.offsetModelSpacePosition.xz;
    let triplanarCoordsZ = TEXTURE_FREQUENCY * fragment.offsetModelSpacePosition.xy;

    // Normalize material weights
    var materialWeights = fragment.materialWeights;
    materialWeights /= materialWeights.x + materialWeights.y + materialWeights.z + materialWeights.w;

    let blendedModelSpaceNormalVector = triplanarSampleAndBlendNormalTextures(
        materialNormalTextures,
        materialSampler,
        modelSpaceNormalVector,
        triplanarWeights,
        triplanarCoordsX,
        triplanarCoordsY,
        triplanarCoordsZ,
        fragment.materialIndices,
        materialWeights,
    );

    let cameraSpaceNormalVector = rotateVectorWithQuaternion(
        fragment.modelToCameraSpaceRotationQuaternion,
        blendedModelSpaceNormalVector,
    );
    output.normalVector = vec4f(convertNormalVectorToNormalColor(cameraSpaceNormalVector), 1.0);

    let color = triplanarSampleAndBlendTextures(
        materialColorTextures,
        materialSampler,
        triplanarWeights,
        triplanarCoordsX,
        triplanarCoordsY,
        triplanarCoordsZ,
        fragment.materialIndices,
        materialWeights,
    ).rgb;
    output.materialColor = vec4f(color, 1.0);

    var roughness = triplanarSampleAndBlendTextures(
        materialRoughnessTextures,
        materialSampler,
        triplanarWeights,
        triplanarCoordsX,
        triplanarCoordsY,
        triplanarCoordsZ,
        fragment.materialIndices,
        materialWeights,
    ).r;

    let materialProperties1 = fixedMaterialProperties[fragment.materialIndices.x];
    let materialProperties2 = fixedMaterialProperties[fragment.materialIndices.y];
    let materialProperties3 = fixedMaterialProperties[fragment.materialIndices.z];
    let materialProperties4 = fixedMaterialProperties[fragment.materialIndices.w];

    let materialProperties = materialWeights.x * materialProperties1
        + materialWeights.y * materialProperties2
        + materialWeights.z * materialProperties3
        + materialWeights.w * materialProperties4;

    let specularReflectance = materialProperties.x;

    // Apply scale factor to sampled roughness and convert to GGX
    roughness *= materialProperties.y;
    roughness = computeGGXRoughnessFromPerceptuallyLinearRoughness(roughness);

    let metalness = materialProperties.z;
    let preExposedEmissiveLuminance = pushConstants.exposure * materialProperties.w;

    output.materialProperties = vec4f(specularReflectance, roughness, metalness, preExposedEmissiveLuminance);

    return output;
}
