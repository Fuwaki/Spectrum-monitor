mod wgpu_app;
mod egui_app;
mod winit_app;
mod audio;
mod compute;
fn main(){
    env_logger::init();
    let mut app=winit_app::App::new();
    app.run();
}