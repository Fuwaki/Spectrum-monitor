// 将历史纹理左移一列，并在最右侧添加新数据
@group(0) @binding(0) var history_tex: texture_2d<f32>;
@group(0) @binding(1) var current_tex: texture_storage_2d<rgba8unorm, write>;
struct SampleData {
    data: array<f32, 4096>,
    length: u32
};

@group(0) @binding(2)
var<storage,read> sampleData: SampleData;
// 简单的伪随机数生成器
fn hash12(p: vec2<f32>) -> vec2<f32> {
    let p_float = vec2<f32>(p);
    let hash_x = fract(sin(dot(p_float, vec2<f32>(127.1, 311.7))) * 43758.5453);
    let hash_y = fract(sin(dot(p_float, vec2<f32>(269.5, 183.3))) * 43758.5453);
    return vec2<f32>(hash_x, hash_y);
}

fn random2d(p: vec2<f32>) -> f32 {
    let hashed = hash12(p);
    return hashed.x;
}
@compute @workgroup_size(32,8)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= textureDimensions(current_tex).xy) { return; }
    let id_f32: vec3<f32> = vec3<f32>(id);
    // textureStore(current_tex,id.xy,vec4<f32>(log(id_f32.x/100+1)/10,0.5,0.9,1.0));
    // 目标坐标：当前像素
    let dst_pixel = id.xy;

    // 如果目标在右侧新列（绘制新频谱条）
    if dst_pixel.x == textureDimensions(history_tex).x - 1 {
        // 在此处生成新频谱数据（示例：随机高度）
        textureStore(current_tex, dst_pixel, vec4(0.0,0.0,sampleData.data[dst_pixel.y], 1.0));
    } else {
        // 从历史纹理的右侧一列采样（实现左移）
        let src_pixel = vec2<u32>(dst_pixel.x + 1, dst_pixel.y);
        let color = textureLoad(history_tex, src_pixel, 0);
        textureStore(current_tex, dst_pixel, color);
    }
}