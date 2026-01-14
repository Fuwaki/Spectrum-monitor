// 将历史纹理左移一列，并在最右侧添加新数据
@group(0) @binding(0) var current_tex: texture_storage_2d<rgba8unorm, write>;
struct SampleData {
    data: array<f32, 16384>,
    length: u32,
    factor: f32,
    write_x: u32,
};
@group(0) @binding(1)
var<storage,read> sampleData: SampleData;
fn hsv2rgb(h: f32, s: f32, v: f32) -> vec3f {

    if s <= 0.0 {
        return vec3f(v, v, v);
    }

    let h_normalized = fract(h); // 确保色相在[0, 1)范围内
    let h_scaled = h_normalized * 6.0;
    let i = floor(h_scaled);
    let f = h_scaled - i;

    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));

    let i_int = u32(i);
    
    // 根据色相区段选择颜色组合
    switch(i_int) {
        case 0u:  { return vec3f(v, t, p); }
        case 1u:  { return vec3f(q, v, p); }
        case 2u:  { return vec3f(p, v, t); }
        case 3u:  { return vec3f(p, q, v); }
        case 4u:  { return vec3f(t, p, v); }
        default:  { return vec3f(v, p, q); } // case 5u
    }
}
fn qwq(g: f32) -> vec3f {
    // 使用指数压缩数据到0-1范围，参数0.15控制压缩程度
    let gray = 1.0 - exp(-1 * sampleData.factor * g);
    
    // 调整色相范围从蓝色(0.6)到红色(0.0)
    let hue = 0.6 * (1.0 - gray);
    
    // 动态饱和度（高值区域略微降低）
    let saturation = mix(1.0, 0.8, smoothstep(0.0, 1.0, gray));
    
    // 使用平方根增强低值亮度
    let value = pow(gray, 0.5);

    return hsv2rgb(hue, saturation, value);
}
@compute @workgroup_size(1,8)


fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= vec2(1, textureDimensions(current_tex).y)) { return; }
    
    // 我们只处理 y 轴，x 轴由 sampleData.write_x 决定
    let dst_pixel = vec2<u32>(sampleData.write_x, id.y);
    
    // 如果 y 超出数据范围，写入黑色或不做处理
    if id.y < sampleData.length {
        textureStore(current_tex, dst_pixel, vec4(qwq(sampleData.data[id.y]), 1.0));
    } else {
        textureStore(current_tex, dst_pixel, vec4(0.0, 0.0, 0.0, 1.0));
    }
}