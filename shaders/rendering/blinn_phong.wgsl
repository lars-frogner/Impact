fn computeDiffuseBlinnPhongReflectedLuminance(
    dots: ReflectionDotProducts,
    albedo: vec3<f32>,
    incidentLuminance: vec3<f32>,
) -> vec3<f32> {
    return incidentLuminance * computeDiffuseBlinnPhongBRDF(dots, albedo);
}

fn computeSpecularBlinnPhongReflectedLuminance(
    dots: ReflectionDotProducts,
    specularReflectance: vec3<f32>,
    shininess: f32,
    incidentLuminance: vec3<f32>,
) -> vec3<f32> {
    return incidentLuminance * computeSpecularBlinnPhongBRDF(dots, specularReflectance, shininess);
}

fn computeBlinnPhongReflectedLuminance(
    dots: ReflectionDotProducts,
    albedo: vec3<f32>,
    specularReflectance: vec3<f32>,
    shininess: f32,
    incidentLuminance: vec3<f32>,
) -> vec3<f32> {
    return incidentLuminance * (computeDiffuseBlinnPhongBRDF(dots, albedo) + computeSpecularBlinnPhongBRDF(dots, specularReflectance, shininess));
}

fn computeDiffuseBlinnPhongBRDF(
    dots: ReflectionDotProducts,
    albedo: vec3<f32>,
) -> vec3<f32> {
    // The factor 0.318309886 is 1 / pi
    return (clampToZero(dots.LDotN) * 0.318309886) * albedo;
}

fn computeSpecularBlinnPhongBRDF(
    dots: ReflectionDotProducts,
    specularReflectance: vec3<f32>,
    shininess: f32,
) -> vec3<f32> {
    // The factor 0.159154943 is 1 / (2 * pi)
    return (0.159154943 * (shininess + 2.0) * pow(clampToZero(dots.NDotH), shininess)) * specularReflectance;
}