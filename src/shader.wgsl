struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
struct ScaleFactor {
    a: f32,
    b: f32,
    l: f32
}
@group(0) @binding(0)
var texture_sampler: sampler;
@group(0) @binding(1)
var texture: texture_2d<f32>;
@group(0) @binding(2)
var<uniform> sf:ScaleFactor;
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



@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // 调整此参数以控制对数刻度程度，a越大低频扩展越明显
    var a = sf.l;
    a *= -1.0;
    // 对y坐标进行缩放 到缩放参数指定的区间内
    let y = sf.a + input.uv.y * (sf.b - sf.a);
    
    // 对Y坐标进行对数变换
    var log_y = 1.0-y;
    //如果a是0就不变换了
    if a != 0.0 {
        log_y=log(a * (1.0 - y) + 1.0) / log(a + 1.0);
    }
    
    
    // 构建新的UV坐标
    let new_uv = vec2<f32>(input.uv.x, log_y);

    var temp = vec4(textureSample(texture, texture_sampler, new_uv + vec2f(0.02, 0.0)));
    // var temp = textureLoad(texture, input.uv, 0);
    return vec4(temp.xyz, 1.0);
    // return vec4(texture,1.0);
}
