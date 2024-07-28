// TODO: Diffuse microfacet BRDF evaluates to zero when
// VDotN <= 0, which may happen for visible
// fragments when using normal mapping, leading to dark artifacts

fn computeNoDiffuseGGXSpecularReflectedLuminance(
    dots: ReflectionDotProducts,
    normalIncidenceSpecularReflectance: vec3<f32>,
    roughness: f32,
    incidentLuminance: vec3<f32>,
) -> vec3<f32> {
    let clampedVDotN = clampToZero(dots.VDotN);
    let clampedLDotN = clampToZero(dots.LDotN);

    let specularBRDFTimesPi = computeSpecularGGXBRDFTimesPi(
        normalIncidenceSpecularReflectance,
        clampedLDotN,
        clampedVDotN,
        dots.LDotH,
        dots.NDotH,
        roughness,
    );

    return computeReflectedLuminanceFromBRDFs(vec3<f32>(0.0, 0.0, 0.0), specularBRDFTimesPi, clampedLDotN, incidentLuminance);
}

fn computeGGXRoughnessFromSampledRoughness(sampledRoughness: f32, roughnessScale: f32) -> f32 {
    // Square sampled roughness (assumed perceptually linear) to get
    // GGX roughness, then apply scaling
    return sampledRoughness * sampledRoughness * roughnessScale;
}

fn computeLambertianDiffuseGGXSpecularReflectedLuminance(
    dots: ReflectionDotProducts,
    albedo: vec3<f32>,
    normalIncidenceSpecularReflectance: vec3<f32>,
    roughness: f32,
    incidentLuminance: vec3<f32>,
) -> vec3<f32> {
    let clampedVDotN = clampToZero(dots.VDotN);
    let clampedLDotN = clampToZero(dots.LDotN);
    
    // The Lambertian BRDF (albedo / pi) must be scaled to
    // account for some of the available light being specularly
    // reflected rather than subsurface scattered (Shirley et al.
    // 1997)
    let diffuseBRDFTimesPi = albedo * computeDiffuseBRDFCorrectionFactorForGGXSpecularReflection(
        normalIncidenceSpecularReflectance,
        clampedLDotN,
        clampedVDotN
    );

    let specularBRDFTimesPi = computeSpecularGGXBRDFTimesPi(
        normalIncidenceSpecularReflectance,
        clampedLDotN,
        clampedVDotN,
        dots.LDotH,
        dots.NDotH,
        roughness,
    );

    return computeReflectedLuminanceFromBRDFs(diffuseBRDFTimesPi, specularBRDFTimesPi, clampedLDotN, incidentLuminance);
}

fn computeGGXDiffuseGGXSpecularReflectedLuminance(
    dots: ReflectionDotProducts,
    albedo: vec3<f32>,
    normalIncidenceSpecularReflectance: vec3<f32>,
    roughness: f32,
    incidentLuminance: vec3<f32>,
) -> vec3<f32> {
    let clampedVDotN = clampToZero(dots.VDotN);
    let clampedLDotN = clampToZero(dots.LDotN);
    
    let diffuseBRDFTimesPi = computeDiffuseGGXBRDFTimesPi(
        albedo,
        normalIncidenceSpecularReflectance,
        clampedLDotN,
        clampedVDotN,
        dots.LDotV,
        dots.NDotH,
        roughness,
    );

    let specularBRDFTimesPi = computeSpecularGGXBRDFTimesPi(
        normalIncidenceSpecularReflectance,
        clampedLDotN,
        clampedVDotN,
        dots.LDotH,
        dots.NDotH,
        roughness,
    );

    return computeReflectedLuminanceFromBRDFs(diffuseBRDFTimesPi, specularBRDFTimesPi, clampedLDotN, incidentLuminance);
}

fn computeGGXDiffuseNoSpecularReflectedLuminance(
    dots: ReflectionDotProducts,
    albedo: vec3<f32>,
    roughness: f32,
    incidentLuminance: vec3<f32>,
) -> vec3<f32> {
    let clampedVDotN = clampToZero(dots.VDotN);
    let clampedLDotN = clampToZero(dots.LDotN);
    
    let zero = vec3<f32>(0.0, 0.0, 0.0);

    let diffuseBRDFTimesPi = computeDiffuseGGXBRDFTimesPi(
        albedo,
        zero,
        clampedLDotN,
        clampedVDotN,
        dots.LDotV,
        dots.NDotH,
        roughness,
    );

    return computeReflectedLuminanceFromBRDFs(diffuseBRDFTimesPi, zero, clampedLDotN, incidentLuminance);
}

fn computeFresnelReflectanceIncidenceFactor(clampedLDotN: f32) -> f32 {
    let oneMinusLDotN = 1.0 - clampedLDotN;
    return oneMinusLDotN * oneMinusLDotN * oneMinusLDotN * oneMinusLDotN * oneMinusLDotN;
}

// Computes Fresnel reflectance using the Schlick approximation.
fn computeFresnelReflectance(
    normalIncidenceSpecularReflectance: vec3<f32>,
    clampedLDotN: f32,
) -> vec3<f32> {
    return normalIncidenceSpecularReflectance + (1.0 - normalIncidenceSpecularReflectance) * computeFresnelReflectanceIncidenceFactor(clampedLDotN);
}

// Evaluates (approximately) the Smith height-correlated masking-shadowing
// function divided by (4 * abs(LDotN) * abs(VDotN)) (Hammon 2017).
fn computeScaledGGXMaskingShadowingFactor(
    clampedLDotN: f32,
    clampedVDotN: f32,
    roughness: f32,
) -> f32 {
    return 0.5 / (mix(
        2.0 * clampedLDotN * clampedVDotN,
        clampedLDotN + clampedVDotN,
        roughness
    ) + 1e-6);
}

// Evaluates the GGX distribution multiplied by pi.
fn evaluateGGXDistributionTimesPi(NDotH: f32, roughness: f32) -> f32 {
    let roughnessSquared = roughness * roughness;
    let denom = 1.0 + NDotH * NDotH * (roughnessSquared - 1.0);
    return f32(NDotH > 0.0) * roughnessSquared / (denom * denom + 1e-6);
}

fn computeDiffuseBRDFCorrectionFactorForGGXSpecularReflection(
    normalIncidenceSpecularReflectance: vec3<f32>,
    clampedLDotN: f32,
    clampedVDotN: f32,
) -> vec3<f32> {
    return 1.05 * (1.0 - normalIncidenceSpecularReflectance) * (1.0 - computeFresnelReflectanceIncidenceFactor(clampedLDotN)) * (1.0 - computeFresnelReflectanceIncidenceFactor(clampedVDotN));
}

// Evaluates a fit to the diffuse BRDF derived from microfacet
// theory using the GGX normal distribution and the Smith
// masking-shadowing function (Hammon 2017).
fn computeDiffuseGGXBRDFTimesPi(
    albedo: vec3<f32>,
    normalIncidenceSpecularReflectance: vec3<f32>,
    clampedLDotN: f32,
    clampedVDotN: f32,
    LDotV: f32,
    NDotH: f32,
    roughness: f32,
) -> vec3<f32> {
    let diffuseBRDFSmoothComponent = computeDiffuseBRDFCorrectionFactorForGGXSpecularReflection(
        normalIncidenceSpecularReflectance,
        clampedLDotN,
        clampedVDotN
    );

    var diffuseBRDFRoughComponent = 0.0;
    if abs(NDotH) > 1e-6 {
        let halfOnePlusLDotV = 0.5 * (1.0 + LDotV);
        diffuseBRDFRoughComponent = halfOnePlusLDotV * (0.9 - 0.4 * halfOnePlusLDotV) * (1.0 + 0.5 / NDotH);
    }

    let diffuseBRDFMultiComponent = 0.3641 * roughness;

    return f32(clampedVDotN > 0.0) * albedo * ((1.0 - roughness) * diffuseBRDFSmoothComponent + roughness * diffuseBRDFRoughComponent + albedo * diffuseBRDFMultiComponent);
}

fn computeSpecularGGXBRDFTimesPi(
    normalIncidenceSpecularReflectance: vec3<f32>,
    clampedLDotN: f32,
    clampedVDotN: f32,
    LDotH: f32,
    NDotH: f32,
    roughness: f32,
) -> vec3<f32> {
    return computeFresnelReflectance(normalIncidenceSpecularReflectance, clampToZero(LDotH)) * computeScaledGGXMaskingShadowingFactor(
        clampedLDotN,
        clampedVDotN,
        roughness
    ) * evaluateGGXDistributionTimesPi(NDotH, roughness);
}

fn computeReflectedLuminanceFromBRDFs(
    diffuseBRDFTimesPi: vec3<f32>,
    specularBRDFTimesPi: vec3<f32>,
    clampedLDotN: f32,
    incidentLuminance: vec3<f32>,
) -> vec3<f32> {
    // The factor 0.318309886 is 1 / pi
    return (diffuseBRDFTimesPi + specularBRDFTimesPi) * (clampedLDotN * 0.318309886) * incidentLuminance;
}
