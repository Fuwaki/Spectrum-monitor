use std::os::linux::raw;

use egui::{viewport, Context};
use egui_wgpu::Renderer;
use egui_winit::State;
use wgpu::Device;

use crate::wgpu_app::WGPUState;

pub struct EguiApp {
    render: Renderer,
    state: State,
}
impl EguiApp {
    pub fn new(
        window: &winit::window::Window,
        device: &egui_wgpu::wgpu::Device,
        output_color_format: egui_wgpu::wgpu::TextureFormat,
        output_depth_format: Option<egui_wgpu::wgpu::TextureFormat>,
        msaa_samples: u32,
    ) -> Self {
        let egui_ctx = Context::default();
        let egui_state = State::new(
            egui_ctx,
            viewport::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            Some(2 * 1024),
        );
        let egui_render = Renderer::new(
            device,
            output_color_format,
            output_depth_format,
            msaa_samples,
            true,
        );

        Self {
            state: egui_state,
            render: egui_render,
        }
    }
    pub fn on_input_event(
        &mut self,
        window: &winit::window::Window,
        event: &winit::event::WindowEvent,
    ) {
        let _ = self.state.on_window_event(window, event);
    }

    fn begin_frame(&mut self, window: &winit::window::Window) {
        let raw_input = self.state.take_egui_input(window);
        self.state.egui_ctx().begin_pass(raw_input);
    }
    fn draw(&mut self) {
        egui::Window::new("Hello Fker!")
            .resizable(true)
            .vscroll(true)
            .default_open(false)
            .show(self.state.egui_ctx(), |ui| {
                ui.label("YIAOYIAOYIAO!");

                if ui.button("Fk Him!").clicked() {
                    println!("boom!")
                }

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Pixels per point: {}",
                        self.state.egui_ctx().pixels_per_point()
                    ));
                });
            });
    }
    fn end_frame_and_draw<'a, 'b>(
        &'a mut self,
        state: &'a WGPUState,
    ) -> impl FnOnce(&mut egui_wgpu::wgpu::CommandEncoder, &'b egui_wgpu::wgpu::TextureView) + 'a
    {
        self.state.egui_ctx().set_pixels_per_point(state.screen_descriptor.pixels_per_point);
        let out = self.state.egui_ctx().end_pass();
        
        let trangles =
             self
            .state
            .egui_ctx()
            .tessellate(out.shapes, state.screen_descriptor.pixels_per_point);
        for (id, image_delta) in out.textures_delta.set {
            //更新texture
            self.render
                .update_texture(&state.device, &state.queue, id, &image_delta);
        }
        move |encoder: &mut egui_wgpu::wgpu::CommandEncoder, view: &egui_wgpu::wgpu::TextureView| {
            //更新buffer
            self.render.update_buffers(
                &state.device,
                &state.queue,
                encoder,
                &trangles,
                &state.screen_descriptor,
            );
            //渲染
            let rpass = encoder.begin_render_pass(&egui_wgpu::wgpu::RenderPassDescriptor {
                color_attachments: &[Some(egui_wgpu::wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: egui_wgpu::wgpu::Operations {
                        load: egui_wgpu::wgpu::LoadOp::Load,
                        store: egui_wgpu::wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                label: Some("egui main render pass"),
                occlusion_query_set: None,
            });
            self.render.render(
                &mut rpass.forget_lifetime(),
                &trangles,
                &state.screen_descriptor,
            );
            for x in &out.textures_delta.free {
                self.render.free_texture(x)
            }
        }
    }
    pub fn update<'a>(
        &'a mut self,
        state: &'a WGPUState,
    ) -> impl FnOnce(&'a mut egui_wgpu::wgpu::CommandEncoder, &'a egui_wgpu::wgpu::TextureView) + 'a
    {
        let window = state.window.clone();
        self.begin_frame(&window);
        self.draw();
        //draw
        self.end_frame_and_draw(state)
    }
}
