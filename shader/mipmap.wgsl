// Adapted from the mipmapping example at https://github.com/gfx-rs/wgpu.

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) textureCoords: vec2<f32>,
};

// Meant to be called with 3 vertex indices: 0, 1, 2.
// Draws one large triangle over the clip space like this:
// (the asterisks represent the clip space bounds)
//-1,1           1,1
// ---------------------------------
// |              *              .
// |              *           .
// |              *        .
// |              *      .
// |              *    . 
// |              * .
// |***************
// |            . 1,-1 
// |          .
// |       .
// |     .
// |   .
// |.
@vertex
fn mainVS(@builtin(vertex_index) vertexIndex: u32) -> VertexOutput {
    var output: VertexOutput;
    
    output.textureCoords = vec2<f32>(
        f32(i32(vertexIndex) / 2) * 2.0,
        f32(i32(vertexIndex) & 1) * 2.0
    );
    
    output.position = vec4<f32>(
        output.textureCoords.x * 2.0 - 1.0,
        1.0 - output.textureCoords.y * 2.0,
        0.0,
        1.0
    );
    
    return output;
}

@group(0)
@binding(0)
var colorTexture: texture_2d<f32>;
@group(0)
@binding(1)
var mipSampler: sampler;

@fragment
fn mainFS(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(colorTexture, mipSampler, vertex.textureCoords);
}
