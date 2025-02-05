use crate::{
    audio::{self, FFTWindow},
    wgpu_app::WGPUState,
};
use audio::Audio;
use egui::{viewport, ComboBox, Context};
use egui_wgpu::Renderer;
use egui_winit::State;
use frame_counter::FrameCounter;

pub struct EguiApp {
    render: Renderer,
    state: State,
    audio_stream: Option<Audio>,
    frame_counter: FrameCounter,
    buffer_remain: usize,
    select_fftwindow: FFTWindow,
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
            audio_stream: None,
            frame_counter: FrameCounter::default(),
            buffer_remain: 0,
            select_fftwindow: FFTWindow::Hanning,
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
        egui::Window::new("Spectrum Monitor Options")
            .resizable(true)
            .vscroll(true)
            .default_open(false)
            // .frame(Frame::default().fill(Color32::from_hex("#10101080").unwrap()))
            .show(self.state.egui_ctx(), |ui| {
                if ui.button("Play").clicked() {
                    self.audio_stream = Some(Audio::new());
                    self.audio_stream.as_mut().unwrap().start();
                }
                if ui.button("Stop").clicked() {
                    self.audio_stream.as_mut().unwrap().stop();
                }
                egui::ComboBox::from_label("Select FFT Window")
                    .selected_text(format!("{:?}", self.select_fftwindow))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.select_fftwindow,
                            FFTWindow::Hanning,
                            "Hanning",
                        );
                        ui.selectable_value(
                            &mut self.select_fftwindow,
                            FFTWindow::Rectangular,
                            "Rectangular",
                        );
                    });

                ui.separator();
                ui.label(format!("fps:{:.2}", self.frame_counter.avg_frame_rate()));
                ui.label(format!("buffer remain:{:.2}", self.buffer_remain));
            });
    }
    pub fn get_audio_stream_data(&mut self) -> Option<(Vec<f32>, usize)> {
        let a = self.audio_stream.as_mut()?.fetch_data();
        self.buffer_remain = a.as_ref()?.1;
        a
    }
    fn end_frame_and_draw<'a, 'b>(
        &'a mut self,
        state: &'a WGPUState,
    ) -> impl FnOnce(&mut egui_wgpu::wgpu::CommandEncoder, &'b egui_wgpu::wgpu::TextureView) + 'a
    {
        self.state
            .egui_ctx()
            .set_pixels_per_point(state.screen_descriptor.pixels_per_point);
        let out = self.state.egui_ctx().end_pass();

        let trangles = self
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
        //更新窗函数
        if let Some(a) =&mut self.audio_stream  {
            a.set_fft_window_func(self.select_fftwindow);
        }
        self.frame_counter.tick();
        let window = state.window.clone();
        self.begin_frame(&window);
        self.draw();
        //draw
        self.end_frame_and_draw(state)
    }
}
/*
其实还有一个思路 就是采样的东西使用sampler来搞 其实搞成对数坐标也是可以做到的 计算着色器那边只要负责把没有采样的图片堆出来就好了
*/
