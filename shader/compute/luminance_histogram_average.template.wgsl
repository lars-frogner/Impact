// Adapted from https://bruop.github.io/exposure/

const BIN_COUNT: u32 = {{bin_count}};
const BIN_COUNT_MINUS_TWO: f32 = f32(BIN_COUNT - 2u);

struct Parameters {
    minLog2Luminance: f32,
    log2LuminanceRange: f32,
    currentFrameWeight: f32,
};

var<push_constant> pixelCount: f32;

@group(0) @binding({{params_binding}}) var<uniform> params: Parameters;
@group(0) @binding({{histogram_binding}}) var<storage, read_write> histogram: array<u32>;
@group(0) @binding({{average_binding}}) var<storage, read_write> average: array<f32>;

var<workgroup> weightedCountBuffer: array<u32, BIN_COUNT>;

@compute @workgroup_size(BIN_COUNT, 1, 1)
fn main(
    @builtin(local_invocation_index) localIndex: u32,
) {
    // Get the count from the histogram buffer
    let countForThisBin = histogram[localIndex];
    weightedCountBuffer[localIndex] = countForThisBin * localIndex;

    workgroupBarrier();

    // Reset the count stored in the buffer for the next pass
    histogram[localIndex] = 0u;

    // This loop will perform a weighted count of the luminance range
    for (var cutoff: u32 = (BIN_COUNT >> 1u); cutoff > 0u; cutoff >>= 1u) {
        if (localIndex < cutoff) {
            weightedCountBuffer[localIndex] += weightedCountBuffer[localIndex + cutoff];
        }
        workgroupBarrier();
    }

    // The final calculation should only be performed by a single thread
    if (localIndex == 0) {
        // Since localIndex == 0, `countForThisBin` holds the number of
        // zero-luminance pixels
        let pixelsWithNonZeroLuminance = max(pixelCount - f32(countForThisBin), 1.0);

        let averageBinIndex = f32(weightedCountBuffer[0]) / pixelsWithNonZeroLuminance;

        let averageNormalizedLog2Luminance = (averageBinIndex - 1.0) / f32(BIN_COUNT - 2u);

        let averageLuminance = exp2((averageNormalizedLog2Luminance * params.log2LuminanceRange) + params.minLog2Luminance);

        let averageLuminanceLastFrame = average[0];
        let weightedAverageLuminance = averageLuminanceLastFrame + (averageLuminance - averageLuminanceLastFrame) * params.currentFrameWeight;
        average[0] = weightedAverageLuminance;
    }
}