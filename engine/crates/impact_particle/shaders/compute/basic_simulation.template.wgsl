struct Immediates {
    particleStartIdx: u32,
    particleCount: u32,
    simulationIdx: u32,
    timeStep: u32,
    timeStepDuration: f32,
}

struct Simulation {
    rateOverTime: RateOverTime,
    colorOverLifetime: ColorOverLifetime,
    scaleOverLifetime: ScaleOverLifetime,
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
    minSpawnAngle: f32,
    maxSpawnAngle: f32,
    minSpawnAngularVelocity: f32,
    maxSpawnAngularVelocity: f32,
    minLifetime: f32,
    maxLifetime: f32,
}

struct RateOverTime {
    t0: f32,
    t1: f32,
    t2: f32,
    t3: f32,
    r0: f32,
    r1: f32,
    r2: f32,
    r3: f32,
}

struct ColorOverLifetime {
    t0: f32,
    t1: f32,
    t2: f32,
    t3: f32,
    c0: vec4f,
    c1: vec4f,
    c2: vec4f,
    c3: vec4f,
}

struct ScaleOverLifetime {
    t0: f32,
    t1: f32,
    t2: f32,
    t3: f32,
    s0: f32,
    s1: f32,
    s2: f32,
    s3: f32,
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

var<immediate> imms: Immediates;

@group({{simulation_and_particle_group}}) @binding({{simulation_binding}})
var<uniform> simulations: array<Simulation, {{max_simulation_count}}>;

@group({{simulation_and_particle_group}}) @binding({{particle_binding}})
var<storage, read_write> particles: array<Particle>;

const WORKGROUP_SIZE: u32 = {{workgroup_size}};

const MAX_F32: f32 = 3.40282347e38;

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

fn evaluateSpawningTime(rot: RateOverTime, particleIdx: u32) -> f32 {
    // Cumulative number of particles emitted at t1, t2 and t3 (rate is
    // implicitly 0 before t0)
    let N1 =      0.5 * (rot.t1 - rot.t0) * (rot.r0 + rot.r1);
    let N2 = N1 + 0.5 * (rot.t2 - rot.t1) * (rot.r1 + rot.r2);
    let N3 = N2 + 0.5 * (rot.t3 - rot.t2) * (rot.r2 + rot.r3);

    let n = f32(particleIdx);

    // If the particle is not emitted until after t3 and the rate after t3 is
    // zero, it is never emitted
    if (n >= N3 && rot.r3 <= 0.0) {
        return MAX_F32;
    }

    let t = array<f32, 5>(rot.t0, rot.t1, rot.t2, rot.t3, MAX_F32);
    let r = array<f32, 5>(rot.r0, rot.r1, rot.r2, rot.r3, rot.r3);
    let Ns = array<f32, 4>(0.0, N1, N2, N3);

    // Determine emission window and place within it
    let k = select(select(select(3, 2, n < N3), 1, n < N2), 0, n < N1);
    let dn = n - Ns[k];

    // Compute slope of emission change across window
    let dt = t[k+1] - t[k];
    let s = select(0.0, (r[k+1] - r[k]) / dt, dt > 0.0);

    // The CDF of rate r(t) is count n(t). Obtain time of emission by evaluating
    // its inverse t(n) for n = particle index.
    let disc = max(0.0, r[k] * r[k] + 2.0 * s * dn);
    let rMax = max(max(rot.r0, rot.r1), max(rot.r2, rot.r3));
    let eps  = max(1e-9 * rMax, 1e-30);
    return t[k] + 2.0 * dn / (r[k] + sqrt(disc) + eps);
}

fn spawnParticle(simulation: Simulation, seed: u32) -> Particle {
    var particle: Particle;

    var hash = seed;

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
    let angleFactor = hashToNormF32(hash);
    let angle = mix(simulation.minSpawnAngle, simulation.maxSpawnAngle, angleFactor);

    hash = pcgHash(hash);
    let angularVelocityFactor = hashToNormF32(hash);
    let angularVelocity = mix(simulation.minSpawnAngularVelocity, simulation.maxSpawnAngularVelocity, angularVelocityFactor);

    hash = pcgHash(hash);
    let lifetimeFactor = hashToNormF32(hash);
    let lifetime = max(1e-3, mix(simulation.minLifetime, simulation.maxLifetime, lifetimeFactor));

    particle.age = 0.0;
    particle.lifetime = lifetime;
    particle.position = simulation.spawnPosition + distance * direction;
    particle.velocity = simulation.spawnVelocity + speed * direction;
    particle.extent = extent;
    particle.angle = angle;
    particle.angularVelocity = angularVelocity;

    particle.color = evaluateColorAtLifetimeFraction(simulation.colorOverLifetime, 0.0);
    particle.scale = evaluateScaleAtLifetimeFraction(simulation.scaleOverLifetime, 0.0);

    return particle;
}

fn evaluateColorAtLifetimeFraction(colt: ColorOverLifetime, lf: f32) -> vec4f {
    let t =   array<f32, 6>(0.0,     colt.t0, colt.t1, colt.t2, colt.t3, MAX_F32);
    let c = array<vec4f, 6>(colt.c0, colt.c0, colt.c1, colt.c2, colt.c3, colt.c3);

    let k = select(select(select(select(4, 3, lf < colt.t3), 2, lf < colt.t2), 1, lf < colt.t1), 0, lf < colt.t0);
    let interp = lf - t[k] / (t[k+1] - t[k]);

    return (1.0 - interp) * c[k] + interp * c[k+1];
}

fn evaluateScaleAtLifetimeFraction(solt: ScaleOverLifetime, lf: f32) -> f32 {
    let t = array<f32, 6>(0.0,     solt.t0, solt.t1, solt.t2, solt.t3, MAX_F32);
    let s = array<f32, 6>(solt.s0, solt.s0, solt.s1, solt.s2, solt.s3, solt.s3);

    let k = select(select(select(select(4, 3, lf < solt.t3), 2, lf < solt.t2), 1, lf < solt.t1), 0, lf < solt.t0);
    let interp = lf - t[k] / (t[k+1] - t[k]);

    return (1.0 - interp) * s[k] + interp * s[k+1];
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

    // The lifetime value will be zero at the start of the simulation, then
    // negative until the particle has been spawned the first time, then
    // positive
    if (particle.lifetime <= 0.0) {
        // Initial spawn

        // In this branch, simulation time is stored in the particle age field
        let time = particle.age;

        // If this is the start of the simulation, generate a spawning time
        // for the particle
        if (time <= 0.0) {
            // Store it as a negative lifetime value so that the initial
            // spawning branch can be triggered again in the next time step if
            // we don't spawn the particle now
            particle.lifetime = -evaluateSpawningTime(simulation.rateOverTime, particleIdx);
        }

        // Decode spawning time
        let spawningTime = -particle.lifetime;

        // If we have passed the spawn time, spawn the particle, otherwise
        // advance simulation time
        if (time >= spawningTime) {
            let seed = globalIdx + pcgHash(imms.timeStep);
            particle = spawnParticle(simulation, seed);
        } else {
            particle.age += imms.timeStepDuration;
        }
    } else if (particle.age >= particle.lifetime) {
        // Respawn
        let seed = globalIdx + pcgHash(imms.timeStep);
        particle = spawnParticle(simulation, seed);
    } else {
        // Update

        let lifetimeFraction = particle.age / particle.lifetime;
        particle.color = evaluateColorAtLifetimeFraction(simulation.colorOverLifetime, lifetimeFraction);
        particle.scale = evaluateScaleAtLifetimeFraction(simulation.scaleOverLifetime, lifetimeFraction);

        particle.position += particle.velocity * imms.timeStepDuration;
        particle.angle += particle.angularVelocity * imms.timeStepDuration;
        particle.age += imms.timeStepDuration;
    }

    particles[particleIdx] = particle;
}
