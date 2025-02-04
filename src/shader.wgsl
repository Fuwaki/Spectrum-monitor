// shader.wgsl
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2(-1.0, -1.0),
        vec2(3.0, -1.0),
        vec2(-1.0, 3.0)
    );
    var uvs = array<vec2<f32>, 3>(
        vec2(0.0, 1.0),
        vec2(2.0, 1.0),
        vec2(0.0, -1.0)
    );

    var output: VertexOutput;
    output.position = vec4(positions[in_vertex_index], 0.0, 1.0);
    output.uv = uvs[in_vertex_index];
    return output;
}

@group(0) @binding(0)
var texture_sampler: sampler;
@group(0) @binding(1)
var texture: texture_2d<f32>;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var temp=vec4(textureSample(texture, texture_sampler, input.uv));
    // var temp = textureLoad(texture, input.uv, 0);
    return vec4(temp.xyz, 1.0);
    // return vec4(texture,1.0);
}
