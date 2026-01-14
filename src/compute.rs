use std::u8;

use crate::wgpu_app::WGPUState;

use egui_wgpu::wgpu::util::{BufferInitDescriptor, DeviceExt};
use egui_wgpu::wgpu::{
    self, BindGroupLayoutEntry, BufferBinding, BufferUsages, Queue, ShaderStages,
};
use egui_wgpu::wgpu::{
    include_wgsl, BindGroupDescriptor, BindGroupEntry, ComputePassDescriptor,
    ComputePipelineDescriptor, Texture, TextureDescriptor, TextureViewDescriptor,
};
const MAX_BUFFER_SIZE: usize = 16384; //记得和计算着色器中保持一致
pub struct Compute {
    texture: wgpu::Texture,
    current_index: u32,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sample_buffer: wgpu::Buffer,
    size: (u32, u32),
}
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SampleData {
    pub data: [f32; MAX_BUFFER_SIZE],
    pub length: u32,
    pub factor: f32,
    pub write_x: u32,
    pub _pad: u32, // Padding to maintain alignment if necessary, though u32+u32+u32+u32 is 16 bytes aligned anyway.
}
impl Compute {
    fn create_texture(state: &WGPUState, label: Option<&str>, height: u32) -> Texture {
        state.device.create_texture(&TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width: state.surface_config.width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    }
    pub fn update_data(&mut self, queue: &Queue, data: &[f32], factor: f32) {
        assert!(data.len() < MAX_BUFFER_SIZE, "数据超过最大缓冲区");
        // write_x should range from 0 to width-1
        // We write to current position, then increment for next time
        let write_pos = self.current_index;
        self.current_index = (self.current_index + 1) % self.size.0;
        
        let d = SampleData {
            data: {
                let mut array = [0.0; MAX_BUFFER_SIZE];
                array[..data.len()].copy_from_slice(data);
                array
            },
            length: data.len() as u32,
            factor,
            write_x: write_pos,
            _pad: 0,
        };
        queue.write_buffer(&self.sample_buffer, 0, bytemuck::bytes_of(&d));
        queue.submit([]);
    }

    pub fn new(state: &WGPUState, height: u32) -> Self {
        //初始化缓冲区的数据
        let data = SampleData {
            data: [0.0; MAX_BUFFER_SIZE],
            length: MAX_BUFFER_SIZE as u32,
            factor: 0.15,
            write_x: 0,
            _pad: 0,
        };

        let texture = Self::create_texture(state, Some("texture"), height);
        let sample_buffer = state.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("AudioSample Buffer"),
            contents: bytemuck::bytes_of(&data), //把数据填进去
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });
        let bind_group_layout =
            state
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("compute_bind_group_layout"),
                    entries: &[
                        // 当前纹理 (Storage)
                        BindGroupLayoutEntry {
                            binding: 0,
                            visibility: ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::StorageTexture {
                                access: wgpu::StorageTextureAccess::WriteOnly,
                                format: wgpu::TextureFormat::Rgba8Unorm,
                                view_dimension: wgpu::TextureViewDimension::D2,
                            },
                            count: None,
                        },
                        // 音频频谱数据
                        BindGroupLayoutEntry {
                            binding: 1,
                            visibility: ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });


        //计算着色器
        let compute_shader = state
            .device
            .create_shader_module(include_wgsl!("draw.wgsl"));

        //计算管线布局
        let pipeline_layout =
            state
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("compute_pipeline_layout"),
                    bind_group_layouts: &[&bind_group_layout], // 使用之前创建的绑定组布局
                    push_constant_ranges: &[],
                });

        //计算管线
        let compute_pipline = state
            .device
            .create_compute_pipeline(&ComputePipelineDescriptor {
                label: Some("shift_pipeline"),
                layout: Some(&pipeline_layout),
                module: &compute_shader,
                entry_point: Some("cs_main"),
                compilation_options: Default::default(),
                cache: None,
            });
        Self {
            texture,
            current_index: 0,
            pipeline: compute_pipline,
            bind_group_layout,
            sample_buffer,
            size: (state.surface_config.width, height), // Store width as well
        }
    }
    pub fn on_resize(&mut self, state: &WGPUState, height: u32) {
        self.size = (state.surface_config.width, height);
        self.texture = Self::create_texture(state, Some("texture"), height);
        // Reset or keep index? Keeping index might cause jump if width changed drastically, but it's simpler.
        // We probably should reset current_index if width changes.
        self.current_index = 0; 
    }
    pub fn update(&mut self, state: &WGPUState, encoder: &mut wgpu::CommandEncoder) {
        let current_view = self.texture.create_view(&TextureViewDescriptor::default());

        let bind_group = state.device.create_bind_group(&BindGroupDescriptor {
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&current_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(BufferBinding {
                        buffer: &self.sample_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
            label: Some("compute_bind_group"),
        });
        const WORKGROUP_SIZE: (u32, u32) = (1, 8); // Modified to 1, 8. Dispatch X is 1.

        //开始计算
        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor::default());
        //设置绑定组
        compute_pass.set_bind_group(0, &bind_group, &[]);
        //设置管线
        compute_pass.set_pipeline(&self.pipeline);
        //分发工作

        /*
        We only dispatch 1 workgroup in X, covering the single column we are writing to.
        */
        let dispatch_x = 1;
        let dispatch_y = (self.size.1 + WORKGROUP_SIZE.1 - 1) / WORKGROUP_SIZE.1;
        compute_pass.dispatch_workgroups(dispatch_x, dispatch_y, 1);
        // Index increment is done in update_data
    }
    pub fn output(&self) -> &egui_wgpu::wgpu::Texture {
        &self.texture
    }
    pub fn get_offset(&self) -> f32 {
        if self.size.0 == 0 { return 0.0; }
        // Return the normalized position of the LAST written column
        let last_index = (self.current_index + self.size.0 - 1) % self.size.0;
        last_index as f32 / self.size.0 as f32
    }
}
