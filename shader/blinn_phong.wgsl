fn computeDiffuseBlinnPhongColor(
    normalVector: vec3<f32>,
    diffuseColor: vec3<f32>,
    lightDirection: vec3<f32>,
    lightRadiance: vec3<f32>,
) -> vec3<f32> {
    let diffuseFactor = max(0.0, dot(lightDirection, normalVector));
    return lightRadiance * diffuseFactor * diffuseColor;
}

fn computeSpecularBlinnPhongColor(
    viewDirection: vec3<f32>,
    normalVector: vec3<f32>,
    specularColor: vec3<f32>,
    shininess: f32,
    lightDirection: vec3<f32>,
    lightRadiance: vec3<f32>,
) -> vec3<f32> {
    let halfVector = normalize((lightDirection + viewDirection));
    let specularFactor = pow(max(0.0, dot(halfVector, normalVector)), shininess);
    return lightRadiance * specularFactor * specularColor;
}

fn computeBlinnPhongColor(
    viewDirection: vec3<f32>,
    normalVector: vec3<f32>,
    diffuseColor: vec3<f32>,
    specularColor: vec3<f32>,
    shininess: f32,
    lightDirection: vec3<f32>,
    lightRadiance: vec3<f32>,
) -> vec3<f32> {
    let halfVector = normalize((lightDirection + viewDirection));
    let diffuseFactor = max(0.0, dot(lightDirection, normalVector));
    let specularFactor = pow(max(0.0, dot(halfVector, normalVector)), shininess);
    return lightRadiance * (diffuseFactor * diffuseColor + specularFactor * specularColor);
}
