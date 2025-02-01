use std::sync::Arc;

use winit::{
    application::ApplicationHandler, dpi::{LogicalSize, PhysicalSize}, event_loop::ControlFlow, window::Window,
};

use crate::wgpu_app::{WGPUState, WGPUAPP};

pub struct App<'a> {
    window: Option<Arc<Window>>,
    app: WGPUAPP<'a>,
}

impl<'a> App<'a> {
    pub fn init_window(& mut self, window: Window) {
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
            .create_window(Window::default_attributes())
            .unwrap();
        self.init_window(window);
    }
    


    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        self.app.on_event(self.window.as_ref().unwrap(), &event);
        println!("{:?}",event);

        match event {
            winit::event::WindowEvent::CloseRequested => event_loop.exit(),
            winit::event::WindowEvent::RedrawRequested => {
                self.app.update();
                self.window.as_mut().unwrap().request_redraw();
            }
            winit::event::WindowEvent::Resized(new_size)=>{
                self.app.on_resize(new_size.width,new_size.height);
            }
            _ => {}
        
        }
    }
}
