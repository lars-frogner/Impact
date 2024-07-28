struct VertexOutput {
    @builtin(position) projectedPosition: vec4<f32>,
    @location(0) frustumFarPlanePoint: vec3<f32>,
}

struct PushConstants {
    inverseWindowDimensions: vec2<f32>,
    frameCounter: u32,
}

struct ProjectionUniform {
    projectionMatrix: mat4x4<f32>,
    frustumFarPlaneCorners: array<vec4<f32>, 4>,
}

struct AmbientOcclusionSamples {
    sampleOffsets: array<vec4<f32>, {{max_samples}}>,
    sampleCount: u32,
    sampleRadius: f32,
    sampleNormalization: f32,
    contrast: f32,
}

struct SampleRotation {
    cosRandomAngle: f32,
    sinRandomAngle: f32,
}

struct FragmentOutput {
    @location(0) occlusion: f32,
}

var<push_constant> pushConstants: PushConstants;

@group({{projection_uniform_group}}) @binding({{projection_uniform_binding}}) var<uniform> projectionUniform: ProjectionUniform;

@group({{linear_depth_texture_group}}) @binding({{linear_depth_texture_binding}}) var linearDepthTexture: texture_2d<f32>;
@group({{linear_depth_texture_group}}) @binding({{linear_depth_sampler_binding}}) var linearDepthSampler: sampler;

@group({{normal_vector_texture_group}}) @binding({{normal_vector_texture_binding}}) var normalVectorTexture: texture_2d<f32>;
@group({{normal_vector_texture_group}}) @binding({{normal_vector_sampler_binding}}) var normalVectorSampler: sampler;

@group({{samples_group}}) @binding({{samples_binding}}) var<uniform> ambientOcclusionSamples: AmbientOcclusionSamples;

fn convertFramebufferPositionToScreenTextureCoords(framebufferPosition: vec4<f32>) -> vec2<f32> {
    return (framebufferPosition.xy * pushConstants.inverseWindowDimensions);
}

// From [0, 1] to [-1, 1]
fn convertNormalColorToNormalizedNormalVector(color: vec3<f32>) -> vec3<f32> {
    return normalize(convertNormalColorToNormalVector(color));
}

// From [0, 1] to [-1, 1]
fn convertNormalColorToNormalVector(color: vec3<f32>) -> vec3<f32> {
    // May require normalization depending on filtering
    return 2.0 * (color - 0.5);
}

fn computeSampleRotation(randomAngle: f32) -> SampleRotation {
    var rotation: SampleRotation;
    rotation.cosRandomAngle = cos(randomAngle);
    rotation.sinRandomAngle = sin(randomAngle);
    return rotation;
}

fn generateRandomAngle(cameraFramebufferPosition: vec4<f32>) -> f32 {
    // Multiply noise factor with 2 * pi to get random angle
    return 6.283185307 * generateInterleavedGradientNoiseFactor(cameraFramebufferPosition);
}

// Returns a random number between 0 and 1 based on the pixel coordinates and
// the current frame counter
fn generateInterleavedGradientNoiseFactor(cameraFramebufferPosition: vec4<f32>) -> f32 {
    let timeOffset = pushConstants.frameCounter % 8u;
    let timeOffsetPosition = vec2<f32>(cameraFramebufferPosition.x + f32(timeOffset), cameraFramebufferPosition.y);
    let magic = vec3<f32>(0.06711056, 0.00583715, 52.9829189);
    return fract(magic.z * fract(dot(magic.xy, timeOffsetPosition)));
}

fn computeAmbientOcclusionSampleValue(
    squaredSampleRadius: f32,
    position: vec3<f32>,
    normalVector: vec3<f32>,
    rotation: SampleRotation,
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
    let sampleTextureCoords = computeTextureCoordsForAmbientOcclusionSample(samplingPosition);

    // Sample view space position of the occluder closest to the camera at the
    // projected position
    let sampledOccluderDepth = textureSampleLevel(linearDepthTexture, linearDepthSampler, sampleTextureCoords, 0.0).x;
    let sampledOccluderPosition = computePositionFromLinearDepthAtScreenTextureCoords(sampledOccluderDepth, sampleTextureCoords);

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

fn computeTextureCoordsForAmbientOcclusionSample(samplingPosition: vec3<f32>) -> vec2<f32> {
    let undividedClipSpaceSamplingPosition = projectionUniform.projectionMatrix * vec4<f32>(samplingPosition, 1.0);
    let horizontalClipSpaceSamplingPosition = undividedClipSpaceSamplingPosition.xy / max(undividedClipSpaceSamplingPosition.w, 1e-6);
    var sampleTextureCoords = 0.5 * (horizontalClipSpaceSamplingPosition + 1.0);
    sampleTextureCoords.y = 1.0 - sampleTextureCoords.y;
    return sampleTextureCoords;
}

// Evaluates the visibility estimator of McGuire et al. (2011), "The Alchemy
// Screen-Space Ambient Obscurance Algorithm"
fn computeOcclusion(summedSampleValues: f32) -> f32 {
    return pow(max(0.0, 1.0 - ambientOcclusionSamples.sampleNormalization * summedSampleValues), ambientOcclusionSamples.contrast);
}

fn computePositionFromLinearDepth(linearDepth: f32, frustumFarPlanePoint: vec3<f32>) -> vec3<f32> {
    return linearDepth * frustumFarPlanePoint;
}

fn computePositionFromLinearDepthAtScreenTextureCoords(linearDepth: f32, textureCoords: vec2<f32>) -> vec3<f32> {
    let frustumFarPlanePoint = mix(
        mix(projectionUniform.frustumFarPlaneCorners[3].xyz, projectionUniform.frustumFarPlaneCorners[2].xyz, textureCoords.x),
        mix(projectionUniform.frustumFarPlaneCorners[0].xyz, projectionUniform.frustumFarPlaneCorners[1].xyz, textureCoords.x),
        textureCoords.y
    );
    return computePositionFromLinearDepth(linearDepth, frustumFarPlanePoint);
}

@vertex 
fn mainVS(
    @builtin(vertex_index) vertexIndex: u32,
    @location({{position_location}}) modelSpacePosition: vec3<f32>
) -> VertexOutput {
    var output: VertexOutput;
    output.projectedPosition = vec4<f32>(modelSpacePosition, 1.0);
    output.frustumFarPlanePoint = projectionUniform.frustumFarPlaneCorners[vertexIndex].xyz;
    return output;
}

@fragment 
fn mainFS(input: VertexOutput) -> FragmentOutput {
    var output: FragmentOutput;

    let squaredSampleRadius = ambientOcclusionSamples.sampleRadius * ambientOcclusionSamples.sampleRadius;

    let textureCoords = convertFramebufferPositionToScreenTextureCoords(input.projectedPosition);

    let depth = textureSampleLevel(linearDepthTexture, linearDepthSampler, textureCoords, 0.0).x;
    let position = computePositionFromLinearDepth(depth, input.frustumFarPlanePoint);

    let normalColor = textureSampleLevel(normalVectorTexture, normalVectorSampler, textureCoords, 0.0);
    let normalVector = convertNormalColorToNormalizedNormalVector(normalColor.rgb);

    let randomAngle = generateRandomAngle(input.projectedPosition);
    let rotation = computeSampleRotation(randomAngle);

    var summedOcclusionSampleValues: f32 = 0.0;

    for (var sampleIdx: u32 = 0u; sampleIdx < ambientOcclusionSamples.sampleCount; sampleIdx++) {
        summedOcclusionSampleValues += computeAmbientOcclusionSampleValue(
            squaredSampleRadius,
            position,
            normalVector,
            rotation,
            ambientOcclusionSamples.sampleOffsets[sampleIdx]
        );
    }

    output.occlusion = computeOcclusion(summedOcclusionSampleValues);
    return output;
}