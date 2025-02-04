use egui_wgpu::wgpu::{util::DeviceExt, Device,Queue};
use egui_wgpu::wgpu;
pub fn load_img(device:&Device,queue:&Queue) -> (egui_wgpu::wgpu::Texture, egui_wgpu::wgpu::TextureView, egui_wgpu::wgpu::Sampler) {
    // 加载图片到纹理
    let img = image::load_from_memory(include_bytes!("../goodcat.png"))
        .unwrap()
        .to_rgba8();

    let texture_size = wgpu::Extent3d {
        width: img.width(),
        height: img.height(),
        depth_or_array_layers: 1,
    };
    //加载图片
    let texture = device.create_texture_with_data(
        &queue,
        &wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            label: Some("example_texture"),
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::MipMajor,
        &img,
    );

    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    (texture,texture_view,sampler)
}
