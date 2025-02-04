use std::u8;

use crate::wgpu_app::WGPUState;
use egui_wgpu::wgpu::{self, BindGroupLayoutEntry, ShaderStages};
use egui_wgpu::wgpu::{
    include_wgsl, BindGroupDescriptor, BindGroupEntry, ComputePass, ComputePassDescriptor,
    ComputePipelineDescriptor, Texture, TextureDescriptor, TextureViewDescriptor,
};

pub struct Compute {
    textures: [wgpu::Texture; 2],
    current_index: u8,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}
impl Compute {
    pub fn new(state: &WGPUState) -> Self {
        let texture_a: egui_wgpu::wgpu::Texture = state.device.create_texture(&TextureDescriptor {
            label: Some("textureA"),
            size: egui_wgpu::wgpu::Extent3d {
                width: state.surface_config.width,
                height: state.surface_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST|wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let texture_b = state.device.create_texture(&TextureDescriptor {
            label: Some("textureB"),
            size: wgpu::Extent3d {
                width: state.surface_config.width,
                height: state.surface_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST|wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let bind_group_layout =
            state
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("compute_bind_group_layout"),
                    entries: &[
                        //第一个是历史纹理 所以readonly
                        BindGroupLayoutEntry {
                            binding: 0,
                            visibility: ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::StorageTexture {
                                access: wgpu::StorageTextureAccess::ReadOnly,
                                format: wgpu::TextureFormat::Rgba8Unorm,
                                view_dimension: wgpu::TextureViewDimension::D2,
                            },
                            count: None,
                        },
                        //第二个是当前纹理
                        BindGroupLayoutEntry {
                            binding: 1,
                            visibility: ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::StorageTexture {
                                access: wgpu::StorageTextureAccess::WriteOnly,
                                format: wgpu::TextureFormat::Rgba8Unorm,
                                view_dimension: wgpu::TextureViewDimension::D2,
                            },
                            count: None,
                        },
                    ],
                });

        //计算着色器
        let compute_shader = state
            .device
            .create_shader_module(include_wgsl!("draw.wgsl"));
        //计算管线
        let compute_pipline = state
            .device
            .create_compute_pipeline(&ComputePipelineDescriptor {
                label: Some("shift_pipeline"),
                layout: None,
                module: &compute_shader,
                entry_point: Some("cs_main"),
                compilation_options: Default::default(),
                cache: None,
            });

        Self {
            textures: [texture_a, texture_b],
            current_index: 0,
            pipeline: compute_pipline,
            bind_group_layout,
        }
    }
    pub fn on_resize(&mut self, state: &WGPUState) {
        self.textures[0] = state.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("TextureA"),
            size: wgpu::Extent3d {
                width:state.surface_config.width,
                height:state.surface_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.textures[1] = state.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("TextureB"),
            size: wgpu::Extent3d {
                width:state.surface_config.width,
                height:state.surface_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
    }
    pub fn update(
        &mut self,
        state: &WGPUState,
        encoder: &mut wgpu::CommandEncoder,
    ) -> &egui_wgpu::wgpu::Texture {
        let current_texture = &self.textures[(self.current_index % 2) as usize];
        let history_texture = &self.textures[(1 - self.current_index % 2) as usize];
        let current_view = current_texture.create_view(&TextureViewDescriptor::default());
        let history_view = history_texture.create_view(&TextureViewDescriptor::default());
        //上面绑定组布局告诉有哪些资源 这个地方是指定实际资源的地方
        let bind_group = state.device.create_bind_group(&BindGroupDescriptor {
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureViewArray(&[&current_view]),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureViewArray(&[&history_view]),
                },
            ],
            label: Some("compute_bind_group"),
        });
        //开始计算
        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor::default());
        //设置绑定组
        compute_pass.set_bind_group(0, &bind_group, &[]);
        //设置管线
        compute_pass.set_pipeline(&self.pipeline);
        //分发工作
        compute_pass.dispatch_workgroups(8, 8, 1);
        self.current_index += 1;
        self.current_index %= 2;
        current_texture
    }
}
