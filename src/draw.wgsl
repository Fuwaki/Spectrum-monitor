// 将历史纹理左移一列，并在最右侧添加新数据
@group(0) @binding(0) var history_tex: texture_2d<f32>;
@group(0) @binding(1) var current_tex: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(8, 8)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    textureStore(current_tex,id.xy,vec4<f32>(0.1,0.2,0.9,1.0));
    // 目标坐标：当前像素
    // let dst_pixel = id.xy;

    // // 如果目标在右侧新列（绘制新频谱条）
    // if dst_pixel.x == textureDimensions(history_tex).x - 1 {
    //     // 在此处生成新频谱数据（示例：随机高度）
    //     let height = ...; // 根据实际频谱数据计算
    //     textureStore(current_tex, dst_pixel, vec4(height, 0.0, 0.0, 1.0));
    // } else {
    //     // 从历史纹理的右侧一列采样（实现左移）
    //     let src_pixel = uint2(dst_pixel.x + 1, dst_pixel.y);
    //     let color = textureLoad(history_tex, src_pixel, 0);
    //     textureStore(current_tex, dst_pixel, color);
    // }
}