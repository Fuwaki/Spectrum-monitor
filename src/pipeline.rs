//! 频谱渲染管线
//! 实现 iced shader::Primitive trait

use iced::widget::shader::{self, Storage, Viewport, wgpu};
use iced::Rectangle;
use wgpu::util::DeviceExt;

const MAX_BUFFER_SIZE: usize = 16384;

/// 传递给 compute shader 的数据结构
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SampleData {
    data: [f32; MAX_BUFFER_SIZE],
    length: u32,
    factor: f32,
    write_x: u32,
    _pad: u32,
}

/// 传递给 render shader 的 uniform 数据
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct RenderUniforms {
    /// Y轴缩放起点
    scale_a: f32,
    /// Y轴缩放终点
    scale_b: f32,
    /// 对数缩放参数
    log_scale: f32,
    /// 纹理偏移 (环形缓冲区)
    offset: f32,
    /// Viewport 信息 (clip_bounds)
    viewport_x: f32,
    viewport_y: f32,
    viewport_width: f32,
    viewport_height: f32,
}

/// 折线图的 uniform 数据
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LineUniforms {
    scale_a: f32,
    scale_b: f32,
    log_scale: f32,
    gain: f32,
    data_length: u32,
    height: u32,
    _pad: [u32; 2],  // 对齐到 16 字节
}

/// 频谱绘制原语，携带每帧的数据
#[derive(Debug)]
pub struct SpectrumPrimitive {
    spectrum_data: Vec<f32>,
    data_length: u32,
    gain_factor: f32,
    log_scale: f32,
    texture_height: u32,
    /// 数据版本号，用于判断是否需要更新纹理
    data_version: u64,
    scale_a: f32,
    scale_b: f32,
}

impl SpectrumPrimitive {
    pub fn new(
        spectrum_data: Vec<f32>,
        data_length: u32,
        gain_factor: f32,
        log_scale: f32,
        texture_height: u32,
        _texture_width: u32,
        data_version: u64,
        scale_a: f32,
        scale_b: f32,
    ) -> Self {
        Self {
            spectrum_data,
            data_length,
            gain_factor,
            log_scale,
            texture_height,
            data_version,
            scale_a,
            scale_b,
        }
    }
}

/// 频谱渲染管线，管理 GPU 资源
pub struct SpectrumPipeline {
    // Compute shader 相关
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group_layout: wgpu::BindGroupLayout,
    sample_buffer: wgpu::Buffer,

    // Render shader 相关
    render_pipeline: wgpu::RenderPipeline,
    render_bind_group_layout: wgpu::BindGroupLayout,
    uniform_buffer: wgpu::Buffer,
    sampler: wgpu::Sampler,

    // 频谱纹理
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
    texture_size: (u32, u32),

    // Bind Groups
    compute_bind_group: wgpu::BindGroup,
    render_bind_group: wgpu::BindGroup,

    // 当前写入位置
    current_write_index: u32,
    // 上一次处理的数据版本号
    last_processed_version: u64,
}

impl SpectrumPipeline {
    fn create_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("spectrum_texture"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn create_compute_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        texture_view: &wgpu::TextureView,
        sample_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute_bind_group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sample_buffer.as_entire_binding(),
                },
            ],
        })
    }

    fn create_render_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        texture_view: &wgpu::TextureView,
        uniform_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render_bind_group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        })
    }
}

impl SpectrumPipeline {
    fn new(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Self {
        // ========== Compute Shader 设置 ==========
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("draw.wgsl").into()),
        });

        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("compute_bind_group_layout"),
                entries: &[
                    // 输出纹理 (storage)
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    // 频谱数据 buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("compute_pipeline_layout"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("compute_pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "cs_main",
        });

        // 初始化 sample buffer
        let initial_data = SampleData {
            data: [0.0; MAX_BUFFER_SIZE],
            length: 0,
            factor: 0.15,
            write_x: 0,
            _pad: 0,
        };
        let sample_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sample_buffer"),
            contents: bytemuck::bytes_of(&initial_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // ========== Render Shader 设置 ==========
        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("render_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("render_bind_group_layout"),
                entries: &[
                    // Sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // Texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Uniforms
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render_pipeline_layout"),
                bind_group_layouts: &[&render_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &render_shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &render_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // Uniform buffer
        let uniforms = RenderUniforms {
            scale_a: 0.0,
            scale_b: 1.0,
            log_scale: 0.5,
            offset: 0.0,
            viewport_x: 0.0,
            viewport_y: 0.0,
            viewport_width: 800.0,
            viewport_height: 600.0,
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uniform_buffer"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("texture_sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // 初始纹理 (将在 prepare 中根据需要重建)
        let (texture, texture_view) = Self::create_texture(device, 800, 512);

        // 创建 Bind Groups
        let compute_bind_group = Self::create_compute_bind_group(
            device,
            &compute_bind_group_layout,
            &texture_view,
            &sample_buffer,
        );

        let render_bind_group = Self::create_render_bind_group(
            device,
            &render_bind_group_layout,
            &sampler,
            &texture_view,
            &uniform_buffer,
        );

        Self {
            compute_pipeline,
            compute_bind_group_layout,
            sample_buffer,
            render_pipeline,
            render_bind_group_layout,
            uniform_buffer,
            sampler,
            texture,
            texture_view,
            compute_bind_group,
            render_bind_group,
            texture_size: (800, 512),
            current_write_index: 0,
            last_processed_version: 0,
        }
    }
}

impl shader::Primitive for SpectrumPrimitive {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        storage: &mut Storage,
        bounds: &Rectangle,
        viewport: &Viewport,
    ) {
        // 如果 Storage 中没有 pipeline，先创建
        if !storage.has::<SpectrumPipeline>() {
            storage.store(SpectrumPipeline::new(device, queue, format));
        }

        let pipeline = storage.get_mut::<SpectrumPipeline>().unwrap();

        let physical_size = viewport.physical_size();
        let new_width = physical_size.width;
        let new_height = self.texture_height;

        // 如果纹理大小改变，重新创建纹理
        if pipeline.texture_size != (new_width, new_height) {
            let (texture, view) = SpectrumPipeline::create_texture(device, new_width, new_height);
            
            // 更新 Bind Groups
            pipeline.compute_bind_group = SpectrumPipeline::create_compute_bind_group(
                device,
                &pipeline.compute_bind_group_layout,
                &view,
                &pipeline.sample_buffer,
            );
            
            pipeline.render_bind_group = SpectrumPipeline::create_render_bind_group(
                device,
                &pipeline.render_bind_group_layout,
                &pipeline.sampler,
                &view,
                &pipeline.uniform_buffer,
            );

            pipeline.texture = texture;
            pipeline.texture_view = view;
            pipeline.texture_size = (new_width, new_height);
            // 初始化为 new_width - 1，这样第一次写入 (current_write_index + 1) % new_width = 0
            // 让最新数据从纹理起始位置开始写入
            pipeline.current_write_index = new_width.saturating_sub(1);
        }

        // 只有在数据版本号变化时才更新（确保同一份数据不会被多次写入）
        let has_new_data = self.data_version != pipeline.last_processed_version && self.data_length > 0;
        if has_new_data {
            // 使用 pipeline 自己管理的 write_index
            // 每次写入前递增，确保连续性
            let write_x = (pipeline.current_write_index + 1) % new_width;
            
            // 更新 sample buffer
            let sample_data = SampleData {
                data: {
                    let mut arr = [0.0; MAX_BUFFER_SIZE];
                    let len = self.spectrum_data.len().min(MAX_BUFFER_SIZE);
                    arr[..len].copy_from_slice(&self.spectrum_data[..len]);
                    arr
                },
                length: self.data_length,
                factor: self.gain_factor,
                write_x,
                _pad: 0,
            };
            queue.write_buffer(&pipeline.sample_buffer, 0, bytemuck::bytes_of(&sample_data));

            // 执行 Compute Shader
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("spectrum_compute_encoder"),
            });
            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                compute_pass.set_pipeline(&pipeline.compute_pipeline);
                compute_pass.set_bind_group(0, &pipeline.compute_bind_group, &[]);
                
                // 为了简单起见，这里假设 Y 轴工作组大小为 8
                let dispatch_y = (new_height + 7) / 8;
                compute_pass.dispatch_workgroups(1, dispatch_y, 1);
            }
            queue.submit(Some(encoder.finish()));

            // 更新 pipeline 的索引以便渲染时使用正确的 offset
            pipeline.current_write_index = write_x;
            // 记录已处理的版本号，防止同一数据被重复写入
            pipeline.last_processed_version = self.data_version;
        }

        // 更新 render uniforms
        // Offset mapping: 
        // 我们希望 UV.x=1 (右边) 显示最新数据 (current_write_index)
        // UV.x=0 (左边) 显示最旧数据
        // 在 shader 中: shifted_x = fract(uv.x + offset)
        // 当 uv.x = 1 时, shifted_x = fract(1 + offset) = fract(offset)
        // 我们希望 fract(offset) = current_write_index / width
        // 所以 offset = current_write_index / width (由于 fract(1+x) = fract(x))
        let offset = if new_width > 0 {
            // current_write_index 指向最新写入的列
            // 让 UV.x=1 映射到 current_write_index
            // shifted_x = fract(1.0 + offset) = fract(offset) 应该等于 current_write_index/width
            // 但由于 fract 会包装，我们让 UV.x=0 -> (current_write_index + 1) / width (最旧)
            // 让 UV.x=1 -> current_write_index / width (最新)
            // 需要: fract(0 + offset) = (current_write_index + 1) / width
            // 所以 offset = (current_write_index + 1) / width
            // 这样 UV.x=1 时: fract(1 + offset) = fract(offset) = (current_write_index + 1) / width
            // 不对，让我重新计算...
            // 
            // 假设 width = 100, current_write_index = 50 (最新数据在第50列)
            // 我们希望: UV.x=0 -> 采样第51列(最旧), UV.x=1 -> 采样第50列(最新)
            // shifted_x = fract(uv.x + offset)
            // 当 uv.x=0: shifted_x = offset -> 映射到纹理 offset * width = 51, 所以 offset = 51/100
            // 当 uv.x=1: shifted_x = fract(1 + offset) = offset -> 还是51? 不对。
            // 
            // 实际上 fract(1 + x) = fract(x)，所以 UV.x=0 和 UV.x=1 映射到同一个位置
            // 这是错误的！我们需要让 UV.x 从 0->1 映射到纹理的一个完整周期
            // 
            // 让我重新理解: shifted_x = fract(uv.x + offset)
            // uv.x 从 0 到 1，所以 shifted_x 从 offset 到 1+offset (然后 fract 到 offset)
            // 这意味着我们采样从 offset 开始的纹理一圈
            // 
            // 如果 offset = (current_write_index + 1) / width:
            // UV.x=0 -> 采样第 (current_write_index + 1) 列 (最旧的数据)
            // UV.x 接近 1 -> 采样接近第 current_write_index 列 (最新的数据)
            // 
            // 这个逻辑是对的！但问题是纹理初始化为黑色，在填满之前会看到黑色
            (pipeline.current_write_index + 1) as f32 / new_width as f32
        } else {
            0.0
        };

        // 计算物理像素坐标 (考虑 DPI 缩放)
        let scale_factor = viewport.scale_factor() as f32;
        let physical_bounds = Rectangle {
            x: bounds.x * scale_factor,
            y: bounds.y * scale_factor,
            width: bounds.width * scale_factor,
            height: bounds.height * scale_factor,
        };

        let uniforms = RenderUniforms {
            scale_a: self.scale_a,
            scale_b: self.scale_b,
            log_scale: self.log_scale,
            offset,
            viewport_x: physical_bounds.x,
            viewport_y: physical_bounds.y,
            viewport_width: physical_bounds.width,
            viewport_height: physical_bounds.height,
        };
        queue.write_buffer(&pipeline.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        storage: &Storage,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        let Some(pipeline) = storage.get::<SpectrumPipeline>() else {
            return;
        };

        // ========== Render Pass: 绘制频谱纹理 ==========
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("spectrum_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_scissor_rect(
            clip_bounds.x,
            clip_bounds.y,
            clip_bounds.width,
            clip_bounds.height,
        );

        render_pass.set_pipeline(&pipeline.render_pipeline);
        render_pass.set_bind_group(0, &pipeline.render_bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

// ==================== 折线图 Primitive ====================

/// 折线图绘制原语
#[derive(Debug)]
pub struct SpectrumLinePrimitive {
    spectrum_data: Vec<f32>,
    data_length: u32,
    gain_factor: f32,
    log_scale: f32,
    scale_a: f32,
    scale_b: f32,
    width: u32,
    height: u32,
}

impl SpectrumLinePrimitive {
    pub fn new(
        spectrum_data: Vec<f32>,
        data_length: u32,
        gain_factor: f32,
        log_scale: f32,
        scale_a: f32,
        scale_b: f32,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            spectrum_data,
            data_length,
            gain_factor,
            log_scale,
            scale_a,
            scale_b,
            width,
            height,
        }
    }
}

/// 折线图渲染管线
pub struct LinePipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    uniform_buffer: wgpu::Buffer,
    spectrum_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl LinePipeline {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("line_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("line.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("line_bind_group_layout"),
            entries: &[
                // Uniforms
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Spectrum data
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("line_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("line_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // 初始化 uniform buffer
        let uniforms = LineUniforms {
            scale_a: 0.0,
            scale_b: 1.0,
            log_scale: 0.5,
            gain: 0.15,
            data_length: 0,
            height: 512,
            _pad: [0; 2],
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("line_uniform_buffer"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // 初始化频谱数据 buffer
        let spectrum_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("line_spectrum_buffer"),
            contents: bytemuck::cast_slice(&[0.0f32; MAX_BUFFER_SIZE]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("line_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: spectrum_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            pipeline,
            bind_group_layout,
            uniform_buffer,
            spectrum_buffer,
            bind_group,
        }
    }
}

impl shader::Primitive for SpectrumLinePrimitive {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        storage: &mut Storage,
        _bounds: &Rectangle,
        _viewport: &Viewport,
    ) {
        // 如果 Storage 中没有 pipeline，先创建
        if !storage.has::<LinePipeline>() {
            storage.store(LinePipeline::new(device, format));
        }

        let pipeline = storage.get_mut::<LinePipeline>().unwrap();

        // 更新 uniform buffer
        let uniforms = LineUniforms {
            scale_a: self.scale_a,
            scale_b: self.scale_b,
            log_scale: self.log_scale,
            gain: self.gain_factor,
            data_length: self.data_length,
            height: self.height,
            _pad: [0; 2],
        };
        queue.write_buffer(&pipeline.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        // 更新频谱数据 buffer
        if self.data_length > 0 {
            let len = (self.data_length as usize).min(self.spectrum_data.len());
            queue.write_buffer(
                &pipeline.spectrum_buffer,
                0,
                bytemuck::cast_slice(&self.spectrum_data[..len]),
            );
        }
    }

    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        storage: &Storage,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        let Some(pipeline) = storage.get::<LinePipeline>() else {
            return;
        };

        // 绘制背景
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("line_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_scissor_rect(
            clip_bounds.x,
            clip_bounds.y,
            clip_bounds.width,
            clip_bounds.height,
        );

        render_pass.set_pipeline(&pipeline.pipeline);
        render_pass.set_bind_group(0, &pipeline.bind_group, &[]);
        
        // 绘制折线 - 使用 LineStrip，每个数据点一个顶点
        if self.data_length > 1 {
            render_pass.draw(0..self.data_length, 0..1);
        }
    }
}
