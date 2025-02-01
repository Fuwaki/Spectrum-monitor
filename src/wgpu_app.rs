use std::sync::Arc;

use crate::egui_app::{self, EguiApp};
use egui_wgpu::{wgpu, ScreenDescriptor};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

pub struct WGPUState<'window> {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub surface: wgpu::Surface<'window>,
    pub window: Arc<Window>,
    pub screen_descriptor: ScreenDescriptor,
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
                    required_features: wgpu::Features::empty(),
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
        surface_config.present_mode=wgpu::PresentMode::AutoVsync;
        surface.configure(&device, &surface_config); //使用设备配置surface
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [surface_config.width, surface_config.height],
            pixels_per_point: window.scale_factor() as f32 * 1.3,
        };


        Self {
            device,
            queue,
            surface_config,
            surface,
            window,
            screen_descriptor,
        }
    }
}
pub struct WGPUAPP<'a> {
    state: Option<WGPUState<'a>>,
    app: Option<EguiApp>,
}
impl<'a> WGPUAPP<'a> {
    pub fn new() -> Self {
        Self {
            state: None,
            app: None,
        }
    }

    pub fn init(&mut self, window: Arc<Window>) {
        //初始化wgpu
        self.state = Some(pollster::block_on(WGPUState::new(window.clone())));
        //然后初始化egui应用
        self.app = Some(EguiApp::new(
            &window,
            &self.state.as_ref().unwrap().device,
            self.state.as_ref().unwrap().surface_config.format,
            None,
            1,
        ));
    }
    pub fn on_event(&mut self, window: &Window, e: &WindowEvent) {
        if let Some(r) = self.app.as_mut() {
            r.on_input_event(window, e);
        }
    }
    pub fn on_resize(&mut self, w: u32, h: u32) {
        self.state.as_mut().unwrap().surface_config.width = w;
        self.state.as_mut().unwrap().surface_config.height = h;
        self.state.as_ref().unwrap().surface.configure(
            &self.state.as_ref().unwrap().device,
            &self.state.as_ref().unwrap().surface_config,
        );
        self.state.as_mut().unwrap().screen_descriptor.size_in_pixels = [
            w,h
        ];
    }
    pub fn update(&mut self) {
        let output = self
            .state
            .as_mut()
            .unwrap()
            .surface
            .get_current_texture()
            .expect("找不到一个output");
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.state.as_mut().unwrap().device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            },
        );
        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.2,
                            b: 0.5,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        self.app
            .as_mut()
            .unwrap()
            .update(self.state.as_mut().unwrap())(&mut encoder, &view);
        self.state
            .as_mut()
            .unwrap()
            .queue
            .submit(std::iter::once(encoder.finish()));
        output.present();
    }
}
