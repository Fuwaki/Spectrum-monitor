use std::sync::Arc;

use crate::{
    compute::Compute,
    egui_app::EguiApp,
};
use egui_wgpu::{
    wgpu::{
        self,
        util::{BufferInitDescriptor, DeviceExt},
        BindGroupLayout, BindGroupLayoutDescriptor, BlendState, Buffer, BufferBinding, ColorWrites,
        PipelineCompilationOptions, RenderPipeline,
    },
    ScreenDescriptor,
};
use winit::{event::WindowEvent, window::Window};
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct scale_factor {
    a: f32,
    b: f32,
    log: f32,
}
pub struct WGPUState<'window> {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub surface: wgpu::Surface<'window>,
    pub window: Arc<Window>,
    pub screen_descriptor: ScreenDescriptor,
    pub pipeline: RenderPipeline,
    pub bindgroup_layout: BindGroupLayout,
    pub buffer: Buffer,
}
impl<'window> WGPUState<'window> {
    async fn new(window: Arc<Window>) -> Self {
        //实例
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface = instance
            .create_surface(window.clone())
            .expect("无法创建surface"); //创建surface
        let power_pref = wgpu::PowerPreference::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase {
                power_preference: power_pref,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("找不到一个adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES, //我不需要在web上运行 而且这样可以帮助我使用writeable的纹理
                    ..Default::default()
                },
                None,
            )
            .await
            .expect("找不到一个device");

        //配置surface
        let size = window.inner_size();

        let mut surface_config = surface
            .get_default_config(&adapter, size.width, size.height)
            .expect("找不到一个surface_config");
        surface_config.present_mode = wgpu::PresentMode::AutoVsync;
        surface.configure(&device, &surface_config); //使用设备配置surface
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [surface_config.width, surface_config.height],
            pixels_per_point: window.scale_factor() as f32,
        };

        // let (_, texture_view, sampler) = load_img(&device, &queue);

        //渲染管线配置
        let bindgroup_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("bindgroup_layout"),
            entries: &[
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
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        //创建buffer 用来传递鼠标的缩放操作
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::bytes_of(&scale_factor {
                a: 0.0,
                b: 1.0,
                log: 0.5,
            }),
        });

        //加载着色器
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[&bindgroup_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        Self {
            device,
            queue,
            surface_config,
            surface,
            window,
            screen_descriptor,
            pipeline,
            bindgroup_layout,
            buffer,
        }
    }
}
pub struct WGPUAPP<'a> {
    state: Option<WGPUState<'a>>,
    appgui: Option<EguiApp>,
    audio_compute: Option<Compute>,
    pub height: u32,
    scale: (f32, f32),
}
impl<'a> WGPUAPP<'a> {
    pub fn new() -> Self {
        Self {
            state: None,
            appgui: None,
            audio_compute: None,
            height: 1024 / 2, //这里要和初始的fftsize的一半保持一致
            scale: (0.0, 1.0),
        }
    }
    pub fn handle_close(&self) {
        if let Some(state) = &self.state {
            // Clean up or release resources here if needed
            state.device.poll(wgpu::Maintain::Wait);
        }

        if let Some(appgui) = &self.appgui {
            // Perform any necessary cleanup for the GUI
        }

        if let Some(audio_compute) = &self.audio_compute {
            // Perform any necessary cleanup for the compute resources
        }

        // Additional actions to handle the close event
    }

    pub fn init(&mut self, window: Arc<Window>) {
        //初始化wgpu
        self.state = Some(pollster::block_on(WGPUState::new(window.clone())));
        //然后初始化egui应用
        self.appgui = Some(EguiApp::new(
            &window,
            &self.state.as_ref().unwrap().device,
            self.state.as_ref().unwrap().surface_config.format,
            None,
            1,
        ));
        self.audio_compute = Some(Compute::new(self.state.as_ref().unwrap(), self.height));
    }
    pub fn on_event(&mut self, window: &Window, e: &WindowEvent) {
        if let Some(r) = self.appgui.as_mut() {
            r.on_input_event(window, e);
        }
    }
    pub fn on_resize(&mut self, w: u32, h: u32) {
        let state = self.state.as_mut().unwrap();
        state.surface_config.width = w;
        state.surface_config.height = h;
        state
            .surface
            .configure(&state.device, &state.surface_config);
        state.screen_descriptor.size_in_pixels = [w, h];
        self.audio_compute
            .as_mut()
            .unwrap()
            .on_resize(state, self.height);
    }
    pub fn set_scale_parameters(&mut self, scale: (f32, f32)) {
        self.scale = scale;
    }
    fn update_scale_parameters(&mut self) {
        let s = scale_factor {
            a: self.scale.0,
            b: self.scale.1,
            log: 1.0 - self.appgui.as_mut().unwrap().log_scale,
        };
        let state = self.state.as_mut().unwrap();
        state
            .queue
            .write_buffer(&state.buffer, 0, bytemuck::bytes_of(&s));
        state.queue.submit([]);
        // println!("{:?}", scale);
    }
    pub fn update(&mut self) {
        self.update_scale_parameters();
        let state = self.state.as_mut().unwrap();
        let output = state
            .surface
            .get_current_texture()
            .expect("找不到一个output");
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        //如果有新数据 上计算pass
        while let Some(d) = self.appgui.as_mut().unwrap().get_audio_stream_data() {
            if self.height != d.1 / 2 as u32 {
                //如果fftsize发生了改变 那么通过计算pass更变高度
                self.height = d.1 / 2 as u32;
                self.audio_compute
                    .as_mut()
                    .unwrap()
                    .on_resize(state, self.height);
            }
            let mut encoder =
                state
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Compute Encoder"),
                    });

            //先更新数据
            self.audio_compute
                .as_mut()
                .unwrap()
                .update_data(&state.queue, d.0.as_slice(), d.2);
            self.audio_compute
                .as_mut()
                .unwrap()
                .update(state, &mut encoder);
            state.queue.submit([encoder.finish()]);
        }
        let mut encoder = state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        //把计算pass得到的纹理变成sampler
        let texture = self.audio_compute.as_mut().unwrap().output();
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = state.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        //创建bindgroup
        let bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &state.bindgroup_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(BufferBinding {
                        buffer: &state.buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
            label: Some("diffuse_bind_group"),
        });
        //然后是渲染的
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.8,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_pipeline(&state.pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
        self.appgui.as_mut().unwrap().update(state)(&mut encoder, &view); //最后渲染的是ui

        state.queue.submit(std::iter::once(encoder.finish())); //提交
        output.present();
        // state.device.poll(wgpu::Maintain::Wait); // 阻塞直到计算完成（仅限非 Web 平台）
    }
}
