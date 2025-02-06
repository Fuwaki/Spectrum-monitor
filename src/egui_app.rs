use crate::{
    audio::{self, FFTWindow},
    wgpu_app::WGPUState,
};
use audio::Audio;
use egui::{viewport, Color32, ComboBox, Context, Frame};
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
    fftsize: u32,
    value_gain_factor:f32,
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
        //设置中文字体
        let mut font = egui::FontDefinitions::default();
        font.font_data.insert(
            "my_font".to_owned(),
            egui::FontData::from_static(include_bytes!("../MiSans-Medium.otf")).into(),
        );
        font.families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "my_font".to_owned());

        // Put my font as last fallback for monospace:
        font.families
            .get_mut(&egui::FontFamily::Monospace)
            .unwrap()
            .push("my_font".to_owned());
        egui_ctx.set_fonts(font);

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
            fftsize: 1024,
            value_gain_factor:0.15
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
        egui::Window::new("频谱监视器选项")
            .resizable(true)
            .vscroll(true)
            .default_open(false)
            .frame(Frame::default().fill(Color32::from_hex("#10101080").unwrap()))
            .show(self.state.egui_ctx(), |ui| {
                egui::Grid::new("my_grid")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("控制");
                        if ui.button("开始").clicked() {
                            self.audio_stream = Some(Audio::new());
                            self.audio_stream.as_mut().unwrap().start();
                        }
                        if ui.button("暂停").clicked() {
                            self.audio_stream.as_mut().unwrap().stop();
                        }
                        ui.end_row();
                        ui.label("FFT 大小");
                        ui.add(
                            egui::Slider::new(&mut self.fftsize, 32..=4096)
                                .logarithmic(true)
                        );
                        ui.end_row();
                        ui.label("值增益系数");
                        ui.add(
                            egui::Slider::new(&mut self.value_gain_factor, 0.1..=0.9)
                                .logarithmic(true)
                        );
                        ui.end_row();
                        ui.label("FFT 窗函数");
                        egui::ComboBox::from_label("")
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
                                ui.selectable_value(
                                    &mut self.select_fftwindow,
                                    FFTWindow::Hamming,
                                    "Hamming",
                                );
                                ui.selectable_value(
                                    &mut self.select_fftwindow,
                                    FFTWindow::Blackman,
                                    "Blackman",
                                );
                            });
                        ui.end_row();

                    });
                ui.separator();
                ui.label(format!("帧率：{:.2}", self.frame_counter.avg_frame_rate()));
                ui.label(format!("未播放缓冲区：{:.2}", self.buffer_remain));
            });
    }
    //咱这个函数返回的元祖的第二个元素是当前的fft大小 第三个是因数
    pub fn get_audio_stream_data(&mut self) -> Option<(Vec<f32>, u32,f32)> {
        let a = self.audio_stream.as_mut()?.fetch_data();
        self.buffer_remain = a.as_ref()?.1;
        Some((a.unwrap().0, self.fftsize,self.value_gain_factor ))
        
        
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
    //更新ui中的参数到audio中
    fn update_argument(&mut self) {
        //确保fftsize是2的整数幂
        let t: u32 = (self.fftsize as f32).log2().round() as u32;
        self.fftsize = 2u32.pow(t);
        if let Some(a) = &mut self.audio_stream {
            //更新窗函数
            a.set_fft_window_func(self.select_fftwindow);
            //更新fftsize
            a.set_fft_size(self.fftsize as usize);
        }
    }
    pub fn update<'a>(
        &'a mut self,
        state: &'a WGPUState,
    ) -> impl FnOnce(&'a mut egui_wgpu::wgpu::CommandEncoder, &'a egui_wgpu::wgpu::TextureView) + 'a
    {
        self.update_argument();
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
