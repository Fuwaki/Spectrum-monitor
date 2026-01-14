struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
struct ScaleFactor {
    a: f32,
    b: f32,
    l: f32,
    offset: f32,
    // Viewport 信息 (物理像素坐标)
    viewport_x: f32,
    viewport_y: f32,
    viewport_width: f32,
    viewport_height: f32,
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
    // input.position 是像素坐标 (物理像素)
    // 计算相对于 viewport 的归一化坐标
    let pixel_x = input.position.x;
    let pixel_y = input.position.y;
    
    // 辅助函数内联：对给定像素偏移采样
    // 4x 超采样: 在像素内部采样 4 个点，手动展开循环（WGSL 不支持变量索引数组）
    var color_sum = vec4<f32>(0.0);
    
    // Sample 1: offset (-0.25, -0.25)
    {
        let sample_pixel_x = pixel_x - 0.25;
        let sample_pixel_y = pixel_y - 0.25;
        let rel_x = (sample_pixel_x - sf.viewport_x) / sf.viewport_width;
        let rel_y = (sample_pixel_y - sf.viewport_y) / sf.viewport_height;
        let y_screen = 1.0 - rel_y;
        let log_space_y = sf.a + y_screen * (sf.b - sf.a);
        var sample_y = log_space_y;
        if sf.l > 0.0 {
            let base = 1.0 + sf.l * 100.0;
            sample_y = (pow(base, log_space_y) - 1.0) / (base - 1.0);
        }
        sample_y = clamp(sample_y, 0.0, 1.0);
        let shifted_x = fract(rel_x + sf.offset);
        color_sum = color_sum + textureSample(texture, texture_sampler, vec2<f32>(shifted_x, sample_y));
    }
    
    // Sample 2: offset (0.25, -0.25)
    {
        let sample_pixel_x = pixel_x + 0.25;
        let sample_pixel_y = pixel_y - 0.25;
        let rel_x = (sample_pixel_x - sf.viewport_x) / sf.viewport_width;
        let rel_y = (sample_pixel_y - sf.viewport_y) / sf.viewport_height;
        let y_screen = 1.0 - rel_y;
        let log_space_y = sf.a + y_screen * (sf.b - sf.a);
        var sample_y = log_space_y;
        if sf.l > 0.0 {
            let base = 1.0 + sf.l * 100.0;
            sample_y = (pow(base, log_space_y) - 1.0) / (base - 1.0);
        }
        sample_y = clamp(sample_y, 0.0, 1.0);
        let shifted_x = fract(rel_x + sf.offset);
        color_sum = color_sum + textureSample(texture, texture_sampler, vec2<f32>(shifted_x, sample_y));
    }
    
    // Sample 3: offset (-0.25, 0.25)
    {
        let sample_pixel_x = pixel_x - 0.25;
        let sample_pixel_y = pixel_y + 0.25;
        let rel_x = (sample_pixel_x - sf.viewport_x) / sf.viewport_width;
        let rel_y = (sample_pixel_y - sf.viewport_y) / sf.viewport_height;
        let y_screen = 1.0 - rel_y;
        let log_space_y = sf.a + y_screen * (sf.b - sf.a);
        var sample_y = log_space_y;
        if sf.l > 0.0 {
            let base = 1.0 + sf.l * 100.0;
            sample_y = (pow(base, log_space_y) - 1.0) / (base - 1.0);
        }
        sample_y = clamp(sample_y, 0.0, 1.0);
        let shifted_x = fract(rel_x + sf.offset);
        color_sum = color_sum + textureSample(texture, texture_sampler, vec2<f32>(shifted_x, sample_y));
    }
    
    // Sample 4: offset (0.25, 0.25)
    {
        let sample_pixel_x = pixel_x + 0.25;
        let sample_pixel_y = pixel_y + 0.25;
        let rel_x = (sample_pixel_x - sf.viewport_x) / sf.viewport_width;
        let rel_y = (sample_pixel_y - sf.viewport_y) / sf.viewport_height;
        let y_screen = 1.0 - rel_y;
        let log_space_y = sf.a + y_screen * (sf.b - sf.a);
        var sample_y = log_space_y;
        if sf.l > 0.0 {
            let base = 1.0 + sf.l * 100.0;
            sample_y = (pow(base, log_space_y) - 1.0) / (base - 1.0);
        }
        sample_y = clamp(sample_y, 0.0, 1.0);
        let shifted_x = fract(rel_x + sf.offset);
        color_sum = color_sum + textureSample(texture, texture_sampler, vec2<f32>(shifted_x, sample_y));
    }
    
    let final_color = color_sum / 4.0;
    return vec4(final_color.xyz, 1.0);
}
