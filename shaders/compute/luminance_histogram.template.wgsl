// Adapted from https://bruop.github.io/exposure/

const THREADS_PER_SIDE: u32 = {{threads_per_side}};
const BIN_COUNT: u32 = THREADS_PER_SIDE * THREADS_PER_SIDE;
const BIN_COUNT_MINUS_TWO: f32 = f32(BIN_COUNT - 2u);

const ZERO_LUMINANCE_THRESHOLD: f32 = 0.005;

struct Parameters {
    minLog2Luminance: f32,
    inverseLog2LuminanceRange: f32,
};

var<push_constant> inverseExposure: f32;

@group({{texture_group}}) @binding({{texture_binding}}) var preExposedLuminanceTexture: texture_2d<f32>;
@group({{params_group}}) @binding({{params_binding}}) var<uniform> params: Parameters;
@group({{histogram_group}}) @binding({{histogram_binding}}) var<storage, read_write> histogram: array<atomic<u32>>;

// Shared histogram buffer used for storing intermediate sums for each workgroup
var<workgroup> workgroupHistogram: array<atomic<u32>, BIN_COUNT>;

const SRGB_TO_LUMINANCE: vec3f = vec3f(0.2125, 0.7154, 0.0721);

fn computeScalarLuminanceFromColor(color: vec3f) -> f32 {
    return dot(SRGB_TO_LUMINANCE, color);
}

fn determineBinIndexForPreExposedLuminanceColor(preExposedLuminanceColor: vec3f) -> u32 {
    let preExposedLuminance = computeScalarLuminanceFromColor(preExposedLuminanceColor);

    let luminance = inverseExposure * preExposedLuminance;

    // Count of zero-luminance pixels is stored in the zeroth bin
    if (luminance < ZERO_LUMINANCE_THRESHOLD) {
        return 0u;
    }

    let normalizedLog2Luminance = clamp(
        (log2(luminance) - params.minLog2Luminance) * params.inverseLog2LuminanceRange,
        0.0, 1.0
    );

    // Map [0, 1] to [1, BIN_COUNT-1]
    return 1u + u32(normalizedLog2Luminance * BIN_COUNT_MINUS_TWO);
}

@compute @workgroup_size(THREADS_PER_SIDE, THREADS_PER_SIDE, 1)
fn main(
    @builtin(global_invocation_id) globalID: vec3u,
    @builtin(local_invocation_index) localIndex: u32,
) {
    // Initialize the bin for this thread to 0
    workgroupHistogram[localIndex] = 0u;
    workgroupBarrier();

    let dim = vec2u(textureDimensions(preExposedLuminanceTexture));

    // Ignore threads that map to areas beyond the bounds the texture
    if (globalID.x < dim.x && globalID.y < dim.y) {
        let preExposedLuminanceColor = textureLoad(preExposedLuminanceTexture, vec2i(globalID.xy), 0).rgb;
        let binIndex = determineBinIndexForPreExposedLuminanceColor(preExposedLuminanceColor);

        // We use an atomic add to ensure we don't write to the same bin in our
        // histogram from two different threads at the same time
        atomicAdd(&workgroupHistogram[binIndex], 1u);
    }

    // Wait for all threads in the workgroup to reach this point before adding
    // our local histogram to the global one
    workgroupBarrier();

    // Technically there's no chance that two threads write to the same bin
    // here, but different work groups might
    atomicAdd(&histogram[localIndex], workgroupHistogram[localIndex]);
}
