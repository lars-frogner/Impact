// Adapted from https://bruop.github.io/exposure/

const BIN_COUNT: u32 = {{bin_count}};

struct Parameters {
    minLog2Luminance: f32,
    log2LuminanceRange: f32,
    currentFrameWeight: f32,
};

var<push_constant> pixelCount: f32;

@group({{params_group}}) @binding({{params_binding}}) var<uniform> params: Parameters;
@group({{histogram_group}}) @binding({{histogram_binding}}) var<storage, read_write> histogram: array<u32>;
@group({{average_group}}) @binding({{average_binding}}) var<storage, read_write> average: array<f32>;

var<workgroup> weightedCountBuffer: array<u32, BIN_COUNT>;

@compute @workgroup_size(BIN_COUNT, 1, 1)
fn main(
    @builtin(local_invocation_index) localIndex: u32,
) {
    // Get the count from the histogram buffer
    let countForThisBin = histogram[localIndex];

    // Weight the count by the index (which indicates luminance) and assign to
    // the corresponding slot in the workgroup count buffer
    weightedCountBuffer[localIndex] = countForThisBin * localIndex;

    workgroupBarrier();

    // Reset the count stored in the histogram for the next pass
    histogram[localIndex] = 0u;

    // This loop will sum the weighted counts into the first slot in the buffer
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
