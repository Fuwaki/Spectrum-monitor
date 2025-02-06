use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition},
    event_loop::ControlFlow,
    window::Window,
};

use crate::wgpu_app::WGPUAPP;

pub struct App<'a> {
    window: Option<Arc<Window>>,
    app: WGPUAPP<'a>,
    scale: (f32, f32), //用来给频谱图作y轴上的缩放显示的
    mouse_position: PhysicalPosition<f64>,
}

impl<'a> App<'a> {
    pub fn init_window(&mut self, window: Window) {
        let window = Arc::new(window);
        let size = (1440, 768);
        let _ = window.request_inner_size(LogicalSize::new(size.0 as f64, size.1 as f64));
        self.window = Some(window);
        self.app.init(self.window.clone().unwrap());
    }
    pub fn new() -> Self {
        Self {
            window: None,
            app: WGPUAPP::new(),
            scale: (0.0, 1.0),
            mouse_position: PhysicalPosition::new(0.0, 0.0),
        }
    }
    pub fn run(&mut self) {
        let event_loop = winit::event_loop::EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(self).expect("运行app失败");
    }
}
impl ApplicationHandler for App<'_> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = event_loop
            .create_window(Window::default_attributes().with_title("Spectrum Monitor"))
            .unwrap();
        self.init_window(window);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        self.app.on_event(self.window.as_ref().unwrap(), &event);
        match event {
            winit::event::WindowEvent::CloseRequested => event_loop.exit(),
            winit::event::WindowEvent::RedrawRequested => {
                self.app.update();
                self.window.as_mut().unwrap().request_redraw();
            }
            winit::event::WindowEvent::Resized(new_size) => {
                self.app.on_resize(new_size.width, new_size.height);
            }
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position = position;
            }
            winit::event::WindowEvent::MouseWheel { delta, .. } => {
                const TOUCHPAD_SENSITIVITY: f32 = 0.1;
                const MOUSEWHEEL_SENSITIVITY: f32 = 0.1;
                const SCALE_SPEED: f32 = 0.1;
                let d: f32;

                match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        d = y * MOUSEWHEEL_SENSITIVITY;
                    }
                    winit::event::MouseScrollDelta::PixelDelta(position) => {
                        d = position.y as f32 * TOUCHPAD_SENSITIVITY;
                    }
                }
                //算出当前单位坐标系下鼠标的y轴坐标 和wgpu中保持一致
                let y=self.mouse_position.y as f32/self.window.as_mut().unwrap().inner_size().height as f32;

                //算出鼠标在缩放后的视窗内的坐标
                let t=self.scale.0+y*(self.scale.1-self.scale.0);
                //缩放的幅度与边界与鼠标y轴的距离成正比
                self.scale.0=self.scale.0+d*t*SCALE_SPEED;
                self.scale.1=self.scale.1-d*(1.0-t)*SCALE_SPEED;
                //把结果的范围限制在0-1
                self.scale.0=self.scale.0.clamp(0.0, 1.0);
                self.scale.1=self.scale.1.clamp(0.0, 1.0);
                // println!("{d} {y} {:?}", self.scale);
                self.app.set_scale_parameters(self.scale);
            }
            _ => {}
        }
    }
}
