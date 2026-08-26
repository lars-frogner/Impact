struct Immediates {
    particleStartIdx: u32,
    particleCount: u32,
    simulationIdx: u32,
    timeStep: u32,
    timeStepDuration: f32,
}

struct Simulation {
    @size(16) spawnPosition: vec3f,
    @size(16) spawnVelocity: vec3f,
    spawnDirection: vec3f,
    minCosPolarSpawnAngle: f32,
    maxCosPolarSpawnAngle: f32,
    minSpawnExtent: f32,
    maxSpawnExtent: f32,
    minSpawnDistance: f32,
    maxSpawnDistance: f32,
    minSpawnSpeed: f32,
    maxSpawnSpeed: f32,
    minLifetime: f32,
    maxLifetime: f32,
}

struct Particle {
    rgbaBaseColor: vec4f,
    position: vec3f,
    remainingTime: f32,
    velocity: vec3f,
    extent: f32,
}

var<immediate> imms: Immediates;

@group({{simulation_and_particle_group}}) @binding({{simulation_binding}})
var<uniform> simulations: array<Simulation, {{max_simulation_count}}>;

@group({{simulation_and_particle_group}}) @binding({{particle_binding}})
var<storage, read_write> particles: array<Particle>;

const WORKGROUP_SIZE: u32 = {{workgroup_size}};

const X_AXIS: vec3f = vec3f(1.0, 0.0, 0.0);
const Y_AXIS: vec3f = vec3f(0.0, 1.0, 0.0);
const Z_AXIS: vec3f = vec3f(0.0, 0.0, 1.0);

// From Jarzynski & Olano (2020) (see
// https://www.reedbeta.com/blog/hash-functions-for-gpu-rendering).
fn pcgHash(input: u32) -> u32 {
    let state = input * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn hashToNormF32(hash: u32) -> f32 {
    // Divide by 2^32
    return f32(hash) * 2.3283064e-10;
}

// See "Finding quaternion representing the rotation from one vector to another"
// on Stack Overflow for explanation
fn rotationBetweenAxes(fromAxis: vec3f, toAxis: vec3f) -> vec4f {
    if (dot(fromAxis, toAxis) <= -0.999999) {
        return vec4f(anyOrthogonal(fromAxis), 0.0);
    }
    let bisector = normalize(fromAxis + toAxis);
    let quaternion = vec4f(cross(fromAxis, bisector), dot(toAxis, bisector));
    return quaternion;
}

fn anyOrthogonal(v: vec3f) -> vec3f {
    let x = abs(v.x);
    let y = abs(v.y);
    let z = abs(v.z);

    let minAxis = select(select(Z_AXIS, Y_AXIS, y < z), select(Z_AXIS, X_AXIS, x < z), x < y);

    return cross(v, minAxis);
}

fn rotateVectorWithQuaternion(quaternion: vec4f, vector: vec3f) -> vec3f {
    let tmp = 2.0 * cross(quaternion.xyz, vector);
    return vector + quaternion.w * tmp + cross(quaternion.xyz, tmp);
}

@compute @workgroup_size(WORKGROUP_SIZE, 1, 1)
fn main(
    @builtin(global_invocation_id) globalID: vec3u,
) {
    let globalIdx = globalID.x;

    if (globalIdx >= imms.particleCount) {
        return;
    }

    let particleIdx = imms.particleStartIdx + globalIdx;
    let simulation = simulations[imms.simulationIdx];
    var particle = particles[particleIdx];

    if (particle.remainingTime > 0.0) {
        particle.position += particle.velocity * imms.timeStepDuration;
        particle.remainingTime -= imms.timeStepDuration;
    } else {
        var hash = globalIdx + pcgHash(imms.timeStep);

        hash = pcgHash(hash);
        let azimuthalAngle = 6.2831853 * hashToNormF32(hash);
        let cosAzimuthalAngle = cos(azimuthalAngle);
        let sinAzimuthalAngle = sin(azimuthalAngle);

        hash = pcgHash(hash);
        let cosPolarAngleFactor = hashToNormF32(hash);
        let cosPolarAngle = mix(simulation.minCosPolarSpawnAngle, simulation.maxCosPolarSpawnAngle, cosPolarAngleFactor);
        let sinPolarAngle = sqrt(1.0 - max(0.0, cosPolarAngle * cosPolarAngle));

        let dirX = cosAzimuthalAngle * sinPolarAngle;
        let dirY = sinAzimuthalAngle * sinPolarAngle;
        let dirZ = cosPolarAngle;

        let localDirection = vec3f(dirX, dirY, dirZ);

        let rotationToSpawnDirection = rotationBetweenAxes(vec3f(0.0, 0.0, 1.0), simulation.spawnDirection);
        let direction = rotateVectorWithQuaternion(rotationToSpawnDirection, localDirection);

        hash = pcgHash(hash);
        let extentFactor = hashToNormF32(hash);
        let extent = mix(simulation.minSpawnExtent, simulation.maxSpawnExtent, extentFactor);

        hash = pcgHash(hash);
        let distanceFactor = hashToNormF32(hash);
        let distance = mix(simulation.minSpawnDistance, simulation.maxSpawnDistance, distanceFactor);

        hash = pcgHash(hash);
        let speedFactor = hashToNormF32(hash);
        let speed = mix(simulation.minSpawnSpeed, simulation.maxSpawnSpeed, speedFactor);

        hash = pcgHash(hash);
        let lifetimeFactor = hashToNormF32(hash);
        let lifetime = mix(simulation.minLifetime, simulation.maxLifetime, lifetimeFactor);

        particle.rgbaBaseColor = vec4f(1.0, 1.0, 1.0, 1.0);
        particle.position = simulation.spawnPosition + distance * direction;
        particle.velocity = simulation.spawnVelocity + speed * direction;
        particle.remainingTime = lifetime;
        particle.extent = extent;
    }

    particles[particleIdx] = particle;
}
