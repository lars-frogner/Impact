// TODO: Diffuse microfacet BRDF evaluates to zero when
// viewDirectionDotNormalVector <= 0, which may happen for visible
// fragments when using normal mapping, leading to dark artifacts

fn computeNoDiffuseGGXSpecularColor(
    viewDirection: vec3<f32>,
    normalVector: vec3<f32>,
    specularColor: vec3<f32>,
    roughness: f32,
    lightDirection: vec3<f32>,
    lightRadiance: vec3<f32>,
) -> vec3<f32> {
    let halfVector = normalize((lightDirection + viewDirection));

    let clampedLightDirectionDotNormalVector = max(0.0, dot(lightDirection, normalVector));
    let clampedViewDirectionDotNormalVector = max(0.0, dot(viewDirection, normalVector));
    let clampedLightDirectionDotHalfVector = max(0.0, dot(lightDirection, halfVector));
    let normalVectorDotHalfVector = dot(normalVector, halfVector);

    let specularBRDFTimesPi = computeSpecularGGXBRDFTimesPi(
        specularColor,
        clampedLightDirectionDotNormalVector,
        clampedViewDirectionDotNormalVector,
        clampedLightDirectionDotHalfVector,
        normalVectorDotHalfVector,
        roughness,
    );

    return computeColorFromBRDFs(vec3<f32>(0.0, 0.0, 0.0), specularBRDFTimesPi, clampedLightDirectionDotNormalVector, lightRadiance);
}

fn computeGGXRoughnessFromSampledRoughness(sampledRoughness: f32, roughnessScale: f32) -> f32 {
    // Square sampled roughness (assumed perceptually linear) to get
    // GGX roughness, then apply scaling
    return sampledRoughness * sampledRoughness * roughnessScale;
}

fn computeLambertianDiffuseGGXSpecularColor(
    viewDirection: vec3<f32>,
    normalVector: vec3<f32>,
    diffuseColor: vec3<f32>,
    specularColor: vec3<f32>,
    roughness: f32,
    lightDirection: vec3<f32>,
    lightRadiance: vec3<f32>,
) -> vec3<f32> {
    let halfVector = normalize((lightDirection + viewDirection));

    let clampedLightDirectionDotNormalVector = max(0.0, dot(lightDirection, normalVector));
    let clampedViewDirectionDotNormalVector = max(0.0, dot(viewDirection, normalVector));
    let clampedLightDirectionDotHalfVector = max(0.0, dot(lightDirection, halfVector));
    let normalVectorDotHalfVector = dot(normalVector, halfVector);

    // The Lambertian BRDF (diffuseColor / pi) must be scaled to
    // account for some of the available light being specularly
    // reflected rather than subsurface scattered (Shirley et al.
    // 1997)
    let diffuseBRDFTimesPi = diffuseColor * computeDiffuseBRDFCorrectionFactorForGGXSpecularReflection(
        specularColor,
        clampedLightDirectionDotNormalVector,
        clampedViewDirectionDotNormalVector
    );

    let specularBRDFTimesPi = computeSpecularGGXBRDFTimesPi(
        specularColor,
        clampedLightDirectionDotNormalVector,
        clampedViewDirectionDotNormalVector,
        clampedLightDirectionDotHalfVector,
        normalVectorDotHalfVector,
        roughness,
    );

    return computeColorFromBRDFs(diffuseBRDFTimesPi, specularBRDFTimesPi, clampedLightDirectionDotNormalVector, lightRadiance);
}

fn computeGGXDiffuseGGXSpecularColor(
    viewDirection: vec3<f32>,
    normalVector: vec3<f32>,
    diffuseColor: vec3<f32>,
    specularColor: vec3<f32>,
    roughness: f32,
    lightDirection: vec3<f32>,
    lightRadiance: vec3<f32>,
) -> vec3<f32> {
    let halfVector = normalize((lightDirection + viewDirection));

    let clampedLightDirectionDotNormalVector = max(0.0, dot(lightDirection, normalVector));
    let clampedViewDirectionDotNormalVector = max(0.0, dot(viewDirection, normalVector));
    let lightDirectionDotViewDirection = dot(lightDirection, viewDirection);
    let clampedLightDirectionDotHalfVector = max(0.0, dot(lightDirection, halfVector));
    let normalVectorDotHalfVector = dot(normalVector, halfVector);

    let diffuseBRDFTimesPi = computeDiffuseGGXBRDFTimesPi(
        diffuseColor,
        specularColor,
        clampedLightDirectionDotNormalVector,
        clampedViewDirectionDotNormalVector,
        lightDirectionDotViewDirection,
        normalVectorDotHalfVector,
        roughness,
    );

    let specularBRDFTimesPi = computeSpecularGGXBRDFTimesPi(
        specularColor,
        clampedLightDirectionDotNormalVector,
        clampedViewDirectionDotNormalVector,
        clampedLightDirectionDotHalfVector,
        normalVectorDotHalfVector,
        roughness,
    );

    return computeColorFromBRDFs(diffuseBRDFTimesPi, specularBRDFTimesPi, clampedLightDirectionDotNormalVector, lightRadiance);
}

fn computeGGXDiffuseNoSpecularColor(
    viewDirection: vec3<f32>,
    normalVector: vec3<f32>,
    diffuseColor: vec3<f32>,
    roughness: f32,
    lightDirection: vec3<f32>,
    lightRadiance: vec3<f32>,
) -> vec3<f32> {
    let halfVector = normalize((lightDirection + viewDirection));

    let clampedLightDirectionDotNormalVector = max(0.0, dot(lightDirection, normalVector));
    let clampedViewDirectionDotNormalVector = max(0.0, dot(viewDirection, normalVector));
    let lightDirectionDotViewDirection = dot(lightDirection, viewDirection);
    let normalVectorDotHalfVector = dot(normalVector, halfVector);

    let zero = vec3<f32>(0.0, 0.0, 0.0);

    let diffuseBRDFTimesPi = computeDiffuseGGXBRDFTimesPi(
        diffuseColor,
        zero,
        clampedLightDirectionDotNormalVector,
        clampedViewDirectionDotNormalVector,
        lightDirectionDotViewDirection,
        normalVectorDotHalfVector,
        roughness,
    );

    return computeColorFromBRDFs(diffuseBRDFTimesPi, zero, clampedLightDirectionDotNormalVector, lightRadiance);
}

fn computeFresnelReflectanceIncidenceFactor(clampedLightDirectionDotNormalVector: f32) -> f32 {
    let oneMinusLDotN = 1.0 - max(0.0, clampedLightDirectionDotNormalVector);
    return oneMinusLDotN * oneMinusLDotN * oneMinusLDotN * oneMinusLDotN * oneMinusLDotN;
}

// Computes Fresnel reflectance using the Schlick approximation.
fn computeFresnelReflectance(
    specularColor: vec3<f32>,
    clampedLightDirectionDotNormalVector: f32,
) -> vec3<f32> {
    return specularColor + (1.0 - specularColor) * computeFresnelReflectanceIncidenceFactor(clampedLightDirectionDotNormalVector);
}

// Evaluates (approximately) the Smith height-correlated
// masking-shadowing function divided by (4 *
// abs(lightDirectionDotNormalVector) *
// abs(viewDirectionDotNormalVector)) (Hammon 2017).
fn computeScaledGGXMaskingShadowingFactor(
    clampedLightDirectionDotNormalVector: f32,
    clampedViewDirectionDotNormalVector: f32,
    roughness: f32,
) -> f32 {
    return 0.5 / (mix(
        2.0 * clampedLightDirectionDotNormalVector * clampedViewDirectionDotNormalVector,
        clampedLightDirectionDotNormalVector + clampedViewDirectionDotNormalVector,
        roughness
    ) + 1e-6);
}

// Evaluates the GGX distribution multiplied by pi.
fn evaluateGGXDistributionTimesPi(normalVectorDotHalfVector: f32, roughness: f32) -> f32 {
    let roughnessSquared = roughness * roughness;
    let denom = 1.0 + normalVectorDotHalfVector * normalVectorDotHalfVector * (roughnessSquared - 1.0);
    return f32(normalVectorDotHalfVector > 0.0) * roughnessSquared / (denom * denom + 1e-6);
}

fn computeDiffuseBRDFCorrectionFactorForGGXSpecularReflection(
    specularColor: vec3<f32>,
    clampedLightDirectionDotNormalVector: f32,
    clampedViewDirectionDotNormalVector: f32,
) -> vec3<f32> {
    return 1.05 * (1.0 - specularColor) * (1.0 - computeFresnelReflectanceIncidenceFactor(clampedLightDirectionDotNormalVector)) * (1.0 - computeFresnelReflectanceIncidenceFactor(clampedViewDirectionDotNormalVector));
}

// Evaluates a fit to the diffuse BRDF derived from microfacet
// theory using the GGX normal distribution and the Smith
// masking-shadowing function (Hammon 2017).
fn computeDiffuseGGXBRDFTimesPi(
    diffuseColor: vec3<f32>,
    specularColor: vec3<f32>,
    clampedLightDirectionDotNormalVector: f32,
    clampedViewDirectionDotNormalVector: f32,
    lightDirectionDotViewDirection: f32,
    normalVectorDotHalfVector: f32,
    roughness: f32,
) -> vec3<f32> {
    let diffuseBRDFSmoothComponent = computeDiffuseBRDFCorrectionFactorForGGXSpecularReflection(
        specularColor,
        clampedLightDirectionDotNormalVector,
        clampedViewDirectionDotNormalVector
    );

    let halfOnePlusLightDirectionDotViewDirection = 0.5 * (1.0 + lightDirectionDotViewDirection);
    let diffuseBRDFRoughComponent = halfOnePlusLightDirectionDotViewDirection * (0.9 - 0.4 * halfOnePlusLightDirectionDotViewDirection) * (1.0 + 0.5 / (normalVectorDotHalfVector + 1e-6));

    let diffuseBRDFMultiComponent = 0.3641 * roughness;

    return f32(clampedViewDirectionDotNormalVector > 0.0) * diffuseColor * ((1.0 - roughness) * diffuseBRDFSmoothComponent + roughness * diffuseBRDFRoughComponent + diffuseColor * diffuseBRDFMultiComponent);
}

fn computeSpecularGGXBRDFTimesPi(
    specularColor: vec3<f32>,
    clampedLightDirectionDotNormalVector: f32,
    clampedViewDirectionDotNormalVector: f32,
    clampedLightDirectionDotHalfVector: f32,
    normalVectorDotHalfVector: f32,
    roughness: f32,
) -> vec3<f32> {
    return computeFresnelReflectance(specularColor, clampedLightDirectionDotHalfVector) * computeScaledGGXMaskingShadowingFactor(
        clampedLightDirectionDotNormalVector,
        clampedViewDirectionDotNormalVector,
        roughness
    ) * evaluateGGXDistributionTimesPi(normalVectorDotHalfVector, roughness);
}

fn computeColorFromBRDFs(
    diffuseBRDFTimesPi: vec3<f32>,
    specularBRDFTimesPi: vec3<f32>,
    clampedLightDirectionDotNormalVector: f32,
    lightRadiance: vec3<f32>,
) -> vec3<f32> {
    return (diffuseBRDFTimesPi + specularBRDFTimesPi) * clampedLightDirectionDotNormalVector * lightRadiance;
}
