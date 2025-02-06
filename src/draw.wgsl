// 将历史纹理左移一列，并在最右侧添加新数据
@group(0) @binding(0) var history_tex: texture_2d<f32>;
@group(0) @binding(1) var current_tex: texture_storage_2d<rgba8unorm, write>;
struct SampleData {
    data: array<f32, 4096>,
    length: u32,
    factor: f32,
};
@group(0) @binding(2)
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
@compute @workgroup_size(32,8)


fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {

    if any(id.xy >= textureDimensions(current_tex).xy) { return; }
    let id_f32: vec3<f32> = vec3<f32>(id);
    // textureStore(current_tex,id.xy,vec4<f32>(log(id_f32.x/100+1)/10,0.5,0.9,1.0));
    // 目标坐标：当前像素
    let dst_pixel = id.xy;

    // 如果目标在右侧新列（绘制新频谱条）
    if dst_pixel.x == textureDimensions(history_tex).x - 1 {
        textureStore(current_tex, dst_pixel, vec4(qwq(sampleData.data[dst_pixel.y]), 1.0));
        // textureStore(current_tex, dst_pixel, vec4(1.0,0.0,0.0, 1.0));
    } else {
        // 从历史纹理的右侧一列采样（实现左移）
        let src_pixel = vec2<u32>(dst_pixel.x + 1, dst_pixel.y);
        let color = textureLoad(history_tex, src_pixel, 0);
        textureStore(current_tex, dst_pixel, color);
    }
}