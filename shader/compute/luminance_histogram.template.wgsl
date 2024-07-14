
const THREADS_PER_SIDE: u32 = {{threads_per_side}};
const BIN_COUNT: u32 = THREADS_PER_SIDE * THREADS_PER_SIDE;
const BIN_COUNT_MINUS_ONE: f32 = f32(BIN_COUNT - 1u);

const EPSILON: f32 = 0.005;

const SRGB_TO_LUMINANCE: vec3f = vec3f(0.2125, 0.7154, 0.0721);

struct Parameters {
    minLog2Luminance: f32,
    inverseLog2LuminanceRange: f32,
};

@group(0) @binding({{params_binding}}) var<uniform> params: Parameters;
@group(0) @binding({{histogram_binding}}) var<storage, read_write> histogram: array<atomic<u32>>;
@group(0) @binding({{texture_binding}}) var luminanceTexture: texture_2d<f32>;

// Shared histogram buffer used for storing intermediate sums for each work group
var<workgroup> histogramShared: array<atomic<u32>, BIN_COUNT>;

fn srgbLuminance(color: vec3f) -> f32 {
  return dot(color, SRGB_TO_LUMINANCE);
}

// For a given color and luminance range, return the histogram bin index
fn colorToBin(color: vec3f, minLog2Luminance: f32, inverseLog2LuminanceRange: f32) -> u32 {
    // Convert our RGB value to Luminance
    let lum = srgbLuminance(color);

    // Avoid taking the log of zero
    if (lum < EPSILON) {
        return 0u;
    }

    // Calculate the log_2 luminance and express it as a value in [0.0, 1.0]
    // where 0.0 represents the minimum luminance, and 1.0 represents the max.
    let logLum = clamp((log2(lum) - minLog2Luminance) * inverseLog2LuminanceRange, 0.0, 1.0);

    // Map [0, 1] to [1, BIN_COUNT]. The zeroth bin is handled by the epsilon check above.
    return u32(logLum * BIN_COUNT_MINUS_ONE + 1.0);
}

@compute @workgroup_size(THREADS_PER_SIDE, THREADS_PER_SIDE, 1)
fn main(
    @builtin(global_invocation_id) globalInvocationID: vec3<u32>,
    @builtin(local_invocation_index) localInvocationIndex: u32,
) {
    // Initialize the bin for this thread to 0
    histogramShared[localInvocationIndex] = 0u;
    workgroupBarrier();

    let dim = vec2<u32>(textureDimensions(luminanceTexture));
    // Ignore threads that map to areas beyond the bounds of our HDR image
    if (globalInvocationID.x < dim.x && globalInvocationID.y < dim.y) {
        let color = textureLoad(luminanceTexture, vec2<i32>(globalInvocationID.xy), 0).rgb;
        let binIndex = colorToBin(color, params.minLog2Luminance, params.inverseLog2LuminanceRange);
        // We use an atomic add to ensure we don't write to the same bin in our
        // histogram from two different threads at the same time.
        atomicAdd(&histogramShared[binIndex], 1u);
    }

    // Wait for all threads in the work group to reach this point before adding our
    // local histogram to the global one
    workgroupBarrier();

    // Technically there's no chance that two threads write to the same bin here,
    // but different work groups might! So we still need the atomic add.
    atomicAdd(&histogram[localInvocationIndex], histogramShared[localInvocationIndex]);
}
