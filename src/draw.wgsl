// 将历史纹理左移一列，并在最右侧添加新数据
@group(0) @binding(0) var history_tex: texture_2d<f32>;
@group(0) @binding(1) var current_tex: texture_storage_2d<rgba8unorm, write>;
struct SampleData {
    data: array<f32, 4096>,
    length: u32
};
@group(0) @binding(2)
var<storage,read> sampleData: SampleData;
fn hsv2rgb(hsv: vec3<f32>) -> vec3<f32> {
    let h = hsv.x * 6.0;
    let s = hsv.y;
    let v = hsv.z;

    let i = floor(h);
    let f = h - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));

    switch u32(i) {
        case 0u: { return vec3(v, t, p); }
        case 1u: { return vec3(q, v, p); }
        case 2u: { return vec3(p, v, t); }
        case 3u: { return vec3(p, q, v); }
        case 4u: { return vec3(t, p, v); }
        default: { return vec3(v, p, q); }
    }
}
fn qwq(gray: f32) -> vec3f {
    // 定义颜色映射
    // 低数值（蓝色）到高数值（红色）的过渡
    let blue = vec3f(0.0, 0.0, 1.0);  // 蓝色
    let red = vec3f(1.0, 0.0, 0.0);   // 红色
    let purple = vec3f(0.5, 0.0, 0.5); // 紫色
    let pink = vec3f(1.0, 0.0, 0.5);  // 粉色

    // 根据灰度值进行颜色插值
    var color = mix(blue, red, gray); // 从蓝色到红色的线性过渡

    // 添加中间颜色过渡（可选）
    let midGray = 0.5; // 中间灰度值
    if gray < midGray {
        // 从蓝色到紫色过渡
        color = mix(blue, purple, gray / midGray);
    } else {
        // 从紫色到红色过渡
        color = mix(purple, red, (gray - midGray) / (1.0 - midGray));
    }

    return color;
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
        // textureStore(current_tex, dst_pixel, vec4(hsv2rgb(vec3<f32>(sampleData.data[dst_pixel.y],0.9,0.7)).xyz, 1.0));
        textureStore(current_tex, dst_pixel, vec4(qwq(sampleData.data[dst_pixel.y]), 1.0));
    } else {
        // 从历史纹理的右侧一列采样（实现左移）
        let src_pixel = vec2<u32>(dst_pixel.x + 1, dst_pixel.y);
        let color = textureLoad(history_tex, src_pixel, 0);
        textureStore(current_tex, dst_pixel, color);
    }
}