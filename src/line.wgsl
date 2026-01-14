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
    smoothing_factor: u32, // 平滑插值因子 (每两个点之间的分段数)
    _pad: u32,
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
fn spectrum_to_color(amplitude_normalized: f32) -> vec3f {
    let gray = clamp(amplitude_normalized, 0.0, 1.0);
    let hue = 0.6 * (1.0 - gray);
    let saturation = mix(1.0, 0.8, smoothstep(0.0, 1.0, gray));
    let brightness = pow(gray, 0.5);
    return hsv2rgb(hue, saturation, brightness);
}

// 将原始振幅转换为归一化的显示值 (使用对数/dB刻度)
fn amplitude_to_normalized(value: f32) -> f32 {
    let ref_value = 1.0 / uniforms.gain;
    let ratio = value / ref_value;
    
    if ratio <= 0.001 {
        return 0.0;
    }
    
    let db = 20.0 * log(ratio) / log(10.0);
    let normalized = (db + 60.0) / 80.0;
    return clamp(normalized, 0.0, 1.0);
}

// 对数变换函数 (Freq -> Log Space Y)
fn apply_log_scale(freq: f32) -> f32 {
    let s = uniforms.log_scale;
    if s <= 0.0 {
        return freq;
    }
    let base = 1.0 + s * 100.0;
    return log(freq * (base - 1.0) + 1.0) / log(base);
}

// 安全获取数据，处理边界
fn get_data_safe(index: i32) -> f32 {
    let len = i32(uniforms.data_length);
    if len == 0 { return 0.0; }
    // 限制在有效范围内，对于首尾点重复边界值
    let idx = clamp(index, 0, len - 1);
    return spectrum_data[u32(idx)];
}

// Catmull-Rom 样条插值
fn catmull_rom(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let v0 = p1;
    let v1 = 0.5 * (p2 - p0);
    let v2 = p0 - 2.5 * p1 + 2.0 * p2 - 0.5 * p3;
    let v3 = -0.5 * p0 + 1.5 * p1 - 1.5 * p2 + 0.5 * p3;
    return max(0.0, ((v3 * t + v2) * t + v1) * t + v0);
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    
    let smoothing = f32(max(uniforms.smoothing_factor, 1u));
    
    // 计算当前处于哪个数据段
    let float_index = f32(vertex_index) / smoothing;
    
    // 检查范围
    if float_index > f32(uniforms.data_length - 1u) {
        output.position = vec4<f32>(2.0, 2.0, 0.0, 1.0);
        output.color = vec4<f32>(0.0);
        return output;
    }
    
    let i = i32(floor(float_index));
    let t = fract(float_index);
    
    // 获取控制点进行插值
    let p0 = get_data_safe(i - 1);
    let p1 = get_data_safe(i);
    let p2 = get_data_safe(i + 1);
    let p3 = get_data_safe(i + 2);
    
    // 插值计算平滑后的振幅值
    let value = catmull_rom(p0, p1, p2, p3, t);
    
    // 计算归一化的频率位置
    let max_index_f = f32(max(uniforms.data_length - 1u, 1u));
    let freq_normalized = float_index / max_index_f;
    
    // 坐标变换
    let log_space_y = apply_log_scale(freq_normalized);
    let visible_y = (log_space_y - uniforms.scale_a) / (uniforms.scale_b - uniforms.scale_a);
    let ndc_y = visible_y * 2.0 - 1.0;
    
    let amplitude = amplitude_to_normalized(value);
    let ndc_x = amplitude * 2.0 - 1.0;
    
    output.position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    
    let rgb = spectrum_to_color(amplitude);
    output.color = vec4<f32>(rgb, 1.0);
    
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
