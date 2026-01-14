// 频谱折线图着色器
// 横轴为振幅，纵轴为频率

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

struct LineUniforms {
    scale_a: f32,      // Y轴缩放起点
    scale_b: f32,      // Y轴缩放终点
    log_scale: f32,    // 对数缩放参数
    gain: f32,         // 增益因子
    data_length: u32,  // 数据长度
    height: u32,       // 视口高度（像素）
}

@group(0) @binding(0)
var<uniform> uniforms: LineUniforms;

@group(0) @binding(1)
var<storage, read> spectrum_data: array<f32>;

// HSV to RGB 转换
fn hsv2rgb(h: f32, s: f32, v: f32) -> vec3f {
    if s <= 0.0 {
        return vec3f(v, v, v);
    }
    let h_normalized = fract(h);
    let h_scaled = h_normalized * 6.0;
    let i = floor(h_scaled);
    let f = h_scaled - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    let i_int = u32(i);
    switch(i_int) {
        case 0u:  { return vec3f(v, t, p); }
        case 1u:  { return vec3f(q, v, p); }
        case 2u:  { return vec3f(p, v, t); }
        case 3u:  { return vec3f(p, q, v); }
        case 4u:  { return vec3f(t, p, v); }
        default:  { return vec3f(v, p, q); }
    }
}

// 将频谱值映射到颜色
fn spectrum_to_color(value: f32) -> vec3f {
    let gray = 1.0 - exp(-1.0 * uniforms.gain * value);
    let hue = 0.6 * (1.0 - gray);
    let saturation = mix(1.0, 0.8, smoothstep(0.0, 1.0, gray));
    let brightness = pow(gray, 0.5);
    return hsv2rgb(hue, saturation, brightness);
}

// 对数变换函数 (Freq -> Screen Y)
fn apply_log_scale(freq: f32) -> f32 {
    let s = uniforms.log_scale;
    if s <= 0.0 {
        return freq;
    }
    let base = 1.0 + s * 100.0;
    // log_base(freq * (base - 1) + 1)
    return log(freq * (base - 1.0) + 1.0) / log(base);
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    
    // 使用 LineStrip 拓扑，每个顶点对应一个数据点
    let freq_index = vertex_index;
    
    if freq_index >= uniforms.data_length {
        // 超出数据范围，放到屏幕外
        output.position = vec4<f32>(2.0, 2.0, 0.0, 1.0);
        output.color = vec4<f32>(0.0);
        return output;
    }
    
    // 获取频谱值
    let value = spectrum_data[freq_index];
    
    // 计算归一化的频率位置 (0.0 = 低频, 1.0 = 高频)
    // 使用 (data_length - 1) 确保最后一个点的 freq_normalized = 1.0
    let max_index = max(uniforms.data_length - 1u, 1u);
    let freq_normalized = f32(freq_index) / f32(max_index);
    
    // 应用对数缩放 (Freq -> Global Screen Y)
    // 必须与 axis.rs 和 shader.wgsl 逻辑一致
    let global_y = apply_log_scale(freq_normalized);
    
    // 应用Y轴视口缩放 (Map Global Y to Viewport Y)
    // scale_a (Bot) -> 0.0, scale_b (Top) -> 1.0
    let visible_y = (global_y - uniforms.scale_a) / (uniforms.scale_b - uniforms.scale_a);
    
    // 转换到NDC坐标
    // Viewport Y: 0.0 (Bottom) -> NDC -1.0
    // Viewport Y: 1.0 (Top)    -> NDC +1.0
    let ndc_y = visible_y * 2.0 - 1.0;
    
    // X坐标: 振幅映射到 [-1, 1]
    // 使用指数压缩
    let amplitude = 1.0 - exp(-uniforms.gain * value);
    // Amplitude 0 -> Left (-1), 1 -> Right (1)
    let ndc_x = amplitude * 2.0 - 1.0;
    
    output.position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    
    // 根据振幅着色
    let rgb = spectrum_to_color(value);
    output.color = vec4<f32>(rgb, 1.0);
    
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
