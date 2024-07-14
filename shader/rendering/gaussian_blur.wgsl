fn computeSingleHorizontalGaussianBlurSampleColor(
    inputTexture: texture_2d<f32>,
    inputSampler: sampler,
    inverseWindowDimensions: vec2<f32>,
    fragmentCoords: vec2<f32>,
    offsetAndWeight: vec4<f32>,
) -> vec4<f32> {
    return computeHorizontalGaussianBlurSampleColor(
        inputTexture,
        inputSampler,
        inverseWindowDimensions,
        fragmentCoords,
        offsetAndWeight.x,
        offsetAndWeight.y
    );
}

fn computeSingleVerticalGaussianBlurSampleColor(
    inputTexture: texture_2d<f32>,
    inputSampler: sampler,
    inverseWindowDimensions: vec2<f32>,
    fragmentCoords: vec2<f32>,
    offsetAndWeight: vec4<f32>,
) -> vec4<f32> {
    return computeVerticalGaussianBlurSampleColor(
        inputTexture,
        inputSampler,
        inverseWindowDimensions,
        fragmentCoords,
        offsetAndWeight.x,
        offsetAndWeight.y
    );
}

fn computeSymmetricHorizontalGaussianBlurSampleColor(
    inputTexture: texture_2d<f32>,
    inputSampler: sampler,
    inverseWindowDimensions: vec2<f32>,
    fragmentCoords: vec2<f32>,
    offsetAndWeight: vec4<f32>,
) -> vec4<f32> {
    let positiveOffsetSampleColor = computeHorizontalGaussianBlurSampleColor(
        inputTexture,
        inputSampler,
        inverseWindowDimensions,
        fragmentCoords,
        offsetAndWeight.x,
        offsetAndWeight.y
    );
    let negativeOffsetSampleColor = computeHorizontalGaussianBlurSampleColor(
        inputTexture,
        inputSampler,
        inverseWindowDimensions,
        fragmentCoords,
        -offsetAndWeight.x,
        offsetAndWeight.y
    );
    return positiveOffsetSampleColor + negativeOffsetSampleColor;
}

fn computeSymmetricVerticalGaussianBlurSampleColor(
    inputTexture: texture_2d<f32>,
    inputSampler: sampler,
    inverseWindowDimensions: vec2<f32>,
    fragmentCoords: vec2<f32>,
    offsetAndWeight: vec4<f32>,
) -> vec4<f32> {
    let positiveOffsetSampleColor = computeVerticalGaussianBlurSampleColor(
        inputTexture,
        inputSampler,
        inverseWindowDimensions,
        fragmentCoords,
        offsetAndWeight.x,
        offsetAndWeight.y
    );
    let negativeOffsetSampleColor = computeVerticalGaussianBlurSampleColor(
        inputTexture,
        inputSampler,
        inverseWindowDimensions,
        fragmentCoords,
        -offsetAndWeight.x,
        offsetAndWeight.y
    );
    return positiveOffsetSampleColor + negativeOffsetSampleColor;
}

fn computeHorizontalGaussianBlurSampleColor(
    inputTexture: texture_2d<f32>,
    inputSampler: sampler,
    inverseWindowDimensions: vec2<f32>,
    fragmentCoords: vec2<f32>,
    offset: f32,
    weight: f32,
) -> vec4<f32> {
    let sampleTextureCoords = (fragmentCoords + vec2(offset, 0.0)) * inverseWindowDimensions;
    let sampledInputColor = textureSampleLevel(inputTexture, inputSampler, sampleTextureCoords, 0.0);
    return sampledInputColor * weight;
}

fn computeVerticalGaussianBlurSampleColor(
    inputTexture: texture_2d<f32>,
    inputSampler: sampler,
    inverseWindowDimensions: vec2<f32>,
    fragmentCoords: vec2<f32>,
    offset: f32,
    weight: f32,
) -> vec4<f32> {
    let sampleTextureCoords = (fragmentCoords + vec2(0.0, offset)) * inverseWindowDimensions;
    let sampledInputColor = textureSampleLevel(inputTexture, inputSampler, sampleTextureCoords, 0.0);
    return sampledInputColor * weight;
}