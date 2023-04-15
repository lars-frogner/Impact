fn computeDiffuseBlinnPhongColor(
    dots: ReflectionDotProducts,
    diffuseColor: vec3<f32>,
    lightRadiance: vec3<f32>,
) -> vec3<f32> {
    return lightRadiance * clampToZero(dots.LDotN) * diffuseColor;
}

fn computeSpecularBlinnPhongColor(
    dots: ReflectionDotProducts,
    specularColor: vec3<f32>,
    shininess: f32,
    lightRadiance: vec3<f32>,
) -> vec3<f32> {
    return lightRadiance * pow(clampToZero(dots.NDotH), shininess) * specularColor;
}

fn computeBlinnPhongColor(
    dots: ReflectionDotProducts,
    diffuseColor: vec3<f32>,
    specularColor: vec3<f32>,
    shininess: f32,
    lightRadiance: vec3<f32>,
) -> vec3<f32> {
    return lightRadiance * (clampToZero(dots.LDotN) * diffuseColor + pow(clampToZero(dots.NDotH), shininess) * specularColor);
}
