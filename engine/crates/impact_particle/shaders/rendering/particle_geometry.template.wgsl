struct Immediates {
    worldToCameraRotation: vec4f,
    prevWorldToCameraRotation: vec4f,
    worldToCameraTranslation: vec3f,
    prevWorldToCameraTranslation: vec3f,
    inverseWindowDimensions: vec2f,
    frameCounter: u32,
    exposure: f32,
    timeStepDuration: f32,
    materialIdx: u32,
}

struct ProjectionUniform {
    projection: mat4x4f,
    frustumFarPlaneCorners: array<vec4f, 4>,
    inverseFarPlaneZ: vec4f,
    jitterOffsets: array<vec4f, {{jitter_count}}>,
}

struct QuadVertex {
    @location({{quad_texture_coords_location}}) textureCoords: vec2f,
}

struct FragmentInput {
    @builtin(position) projectedPosition: vec4f,
    @location(0) previousClipSpacePosition: vec4f,
    @location(1) cameraSpacePosition: vec3f,
    @location(2) @interpolate(flat) baseColor: vec4f,
    @location(3) @interpolate(flat) materialProperties: vec4f,
    @location(4) textureCoords: vec2f,
    @location(5) @interpolate(flat) particleIdx: u32,
    @location(6) @interpolate(flat) normalAndAlphaTextureIdx: u32,
}

struct FragmentOutput {
    @location(0) linearDepth: f32,
    @location(1) normalVector: vec4f,
    @location(2) motionVector: vec2f,
    @location(3) materialColor: vec4f,
    @location(4) materialProperties: vec4f,
}

struct FixedMaterialProperties {
    @align(16) specularReflectance: f32,
    roughness: f32,
    metalness: f32,
    emissiveLuminance: f32,
    normalAndAlphaTextureIdx: u32,
}

struct Particle {
    color: vec4f,
    position: vec3f,
    age: f32,
    velocity: vec3f,
    lifetime: f32,
    extent: f32,
    scale: f32,
    angle: f32,
    angularVelocity: f32,
}

const JITTER_COUNT: u32 = {{jitter_count}};

const NOISE_TEXTURE_DIMS: vec2u = vec2u({{noise_texture_width}}, {{noise_texture_height}});
const NOISE_TEXTURE_LAYERS: u32 = {{noise_texture_layers}};

var<immediate> imms: Immediates;

@group({{projection_uniform_group}}) @binding({{projection_uniform_binding}})
var<uniform> projectionUniform: ProjectionUniform;

@group({{material_group}}) @binding({{fixed_material_uniform_binding}})
var<uniform> fixedMaterialProperties: array<FixedMaterialProperties, {{n_particle_materials}}>;

@group({{material_group}}) @binding({{normal_and_alpha_texture_array_binding}})
var normalAndAlphaTextures: texture_2d_array<f32>;

@group({{material_group}}) @binding({{normal_and_alpha_sampler_binding}})
var normalAndAlphaSampler: sampler;

@group({{material_group}}) @binding({{noise_texture_array_binding}})
var noiseTextures: texture_2d_array<f32>;

@group({{particle_group}}) @binding({{particle_binding}})
var<storage, read> particles: array<Particle>;

fn convertQuadTextureCoordsToPositionOffsets(textureCoords: vec2f) -> vec3f {
    return vec3f(textureCoords.x - 0.5, 0.5 - textureCoords.y, 0.0);
}

fn rotateQuadTextureCoords(textureCoords: vec2f, angle: f32) -> vec2f {
    let cosAngle = cos(angle);
    let sinAngle = sin(angle);
    let rotation = mat2x2f(cosAngle, sinAngle, -sinAngle, cosAngle);
    return rotation * (textureCoords - vec2f(0.5, 0.5)) + vec2f(0.5, 0.5);
}

fn transformPosition(
    rotationQuaternion: vec4f,
    translation: vec3f,
    position: vec3f
) -> vec3f {
    return rotateVectorWithQuaternion(rotationQuaternion, position) + translation;
}

fn rotateVectorWithQuaternion(quaternion: vec4f, vector: vec3f) -> vec3f {
    let tmp = 2.0 * cross(quaternion.xyz, vector);
    return vector + quaternion.w * tmp + cross(quaternion.xyz, tmp);
}

fn obtainProjectionMatrix() -> mat4x4f {
    var matrix = projectionUniform.projection;
    let jitterIndex = imms.frameCounter % JITTER_COUNT;
    let jitterOffsets = projectionUniform.jitterOffsets[jitterIndex];
    matrix[2][0] += jitterOffsets.x * imms.inverseWindowDimensions.x;
    matrix[2][1] += jitterOffsets.y * imms.inverseWindowDimensions.y;
    return matrix;
}

fn computeGGXRoughnessFromPerceptuallyLinearRoughness(linearRoughness: f32) -> f32 {
    return linearRoughness * linearRoughness;
}

fn convertFramebufferPositionToScreenTextureCoords(framebufferPosition: vec4f) -> vec2<f32> {
    return framebufferPosition.xy * imms.inverseWindowDimensions;
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

@vertex
fn mainVS(
    @builtin(instance_index) particleIdx: u32,
    quadVertex: QuadVertex,
) -> FragmentInput {
    var output: FragmentInput;

    let particle = particles[particleIdx];
    let materialProps = fixedMaterialProperties[imms.materialIdx];

    let cameraSpaceVertexOffset = (particle.scale * particle.extent) * convertQuadTextureCoordsToPositionOffsets(quadVertex.textureCoords);

    let cameraSpaceParticlePosition = transformPosition(imms.worldToCameraRotation, imms.worldToCameraTranslation, particle.position);
    let cameraSpaceVertexPosition = cameraSpaceParticlePosition + cameraSpaceVertexOffset;

    let projectionMatrix = obtainProjectionMatrix();

    output.projectedPosition = projectionMatrix * vec4f(cameraSpaceVertexPosition, 1.0);
    output.cameraSpacePosition = cameraSpaceVertexPosition;

    let prevParticlePosition = particle.position - particle.velocity * imms.timeStepDuration;
    let prevCameraSpaceParticlePosition = transformPosition(imms.prevWorldToCameraRotation, imms.prevWorldToCameraTranslation, prevParticlePosition);
    let prevCameraSpaceVertexPosition = prevCameraSpaceParticlePosition + cameraSpaceVertexOffset;

    output.previousClipSpacePosition = projectionMatrix * vec4f(prevCameraSpaceVertexPosition, 1.0);

    output.baseColor = particle.color;
    output.particleIdx = particleIdx;

    let specularReflectance = materialProps.specularReflectance;
    let roughness = computeGGXRoughnessFromPerceptuallyLinearRoughness(materialProps.roughness);
    let metalness = materialProps.metalness;
    let preExposedEmissiveLuminance = materialProps.emissiveLuminance * imms.exposure;

    output.materialProperties = vec4f(specularReflectance, roughness, metalness, preExposedEmissiveLuminance);

    output.textureCoords = rotateQuadTextureCoords(quadVertex.textureCoords, particle.angle);
    output.normalAndAlphaTextureIdx = materialProps.normalAndAlphaTextureIdx;

    return output;
}

@fragment
fn mainFS(fragment: FragmentInput) -> FragmentOutput {
    var output: FragmentOutput;

    let normalColorAndAlpha = textureSample(
        normalAndAlphaTextures,
        normalAndAlphaSampler,
        fragment.textureCoords,
        fragment.normalAndAlphaTextureIdx,
    );

    let alpha = normalColorAndAlpha.a * fragment.baseColor.a;

    let randomFraction = textureLoad(
        noiseTextures,
        vec2u(fragment.projectedPosition.xy) % NOISE_TEXTURE_DIMS,
        imms.frameCounter % NOISE_TEXTURE_LAYERS,
        0,
    ).r;

    // Apply Cranley-Patterson rotation with golden ratio to decorrelate across
    // particles
    let decorrelatedFraction = fract(randomFraction + f32(fragment.particleIdx) * 0.61803399);

    if (alpha < decorrelatedFraction) {
        discard;
    }

    output.linearDepth = projectionUniform.inverseFarPlaneZ.x * fragment.cameraSpacePosition.z;

    // The billboard is view plane-aligned, so the sampled normal vector is
    // already in camera space. If we ever change from view plane-aligned to
    // viewpoint-oriented billboarding, remember to transform the sampled normal
    // vector.
    output.normalVector = vec4f(normalColorAndAlpha.rgb, 1.0);

    let screenTextureCoords = convertFramebufferPositionToScreenTextureCoords(
        fragment.projectedPosition,
    );
    output.motionVector = computeMotionVector(screenTextureCoords, fragment.previousClipSpacePosition);

    output.materialColor = vec4f(fragment.baseColor.rgb, 1.0);

    output.materialProperties = fragment.materialProperties;

    return output;
}
