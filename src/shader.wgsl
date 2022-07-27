struct CameraUniform {
    view_proj: mat4x4<f32>;
};

struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] color:    vec3<f32>;
};

struct FragmentInput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]]       color:         vec3<f32>;
};

struct FragmentOutput {
    [[location(0)]] color: vec4<f32>;
};

[[group(0), binding(0)]]
var<uniform> camera: CameraUniform;

[[stage(vertex)]]
fn vs_main(model: VertexInput) -> FragmentInput {
    var out: FragmentInput;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    out.color = model.color;
    return out;
}

[[stage(fragment)]]
fn fs_main(in: FragmentInput) -> FragmentOutput {
    var out: FragmentOutput;
    out.color = vec4<f32>(in.color, 1.0);
    return out;
}
