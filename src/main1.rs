use std::{sync::{mpsc, Arc}, thread, time::Duration, vec};

use rustfft::{num_complex::ComplexFloat, FftPlanner};
use tokio::sync::Notify;
mod audio;
mod wgpu_app;
mod egui_app;
mod winit_app;
const SIZE:usize=1024;
fn do_fft(pcm_data: &[f32]) {
    // Perform a forward FFT of size 1234
    use rustfft::{num_complex::Complex, FftPlanner};

    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(pcm_data.len());

    // let mut buffer = vec![Complex { re: 0.0, im: 0.0 }; 1234];
    let mut buffer = pcm_data
        .into_iter()
        .map(|item| Complex { re: *item, im: 0.0 });
    let mut b=Vec::from_iter(buffer);
    // println!("{:?}",b);


    fft.process(&mut b);
    // println!("{:+.2}", b[0].re);
    let effective=&b[0..b.len()/2+1];
    let magnitudes:Vec<f32>=effective.iter().map(|item|item.abs()).collect();
    println!("{:?}",magnitudes);
}
#[tokio::main]
async fn main() {
    // do_fft(&[1.0,2.0,3.0,2.0,1.0,0.0,-1.0]);
    // let native_options = eframe::NativeOptions::default();
    // let _ = eframe::run_native(
    //     "Music",
    //     native_options,
    //     Box::new(|cc| Ok(Box::new(MyEguiApp::new(cc)))),
    // );
    let (tx, rx) = mpsc::channel::<Vec<f32>>();
    let (tx1,rx1)=mpsc::channel::<[f32;SIZE]>();
    let b = true;
    let notify = Arc::new(Notify::new());
    tokio::spawn(audio::run(tx, notify.clone()));
    let chunking: thread::JoinHandle<()>=thread::spawn(move ||{
        let temp=Vec::<f32>::new();
        loop{
            
            if let Ok(mut msg)=rx.recv(){
                while temp.len()>SIZE{
                    let a:Vec<f32>=msg.drain(0..SIZE).collect();
                    let b:&[f32;SIZE]=&a[..].try_into().unwrap();
                    tx1.send(*b);
                }

            }

        }
    });


    loop {
        if let Ok(msg) = rx1.recv() {
            // println!("{:?}", do_fft(&msg));
            do_fft(&msg);
            // println!("{}",msg.len());
        }
        // println!("waiting");
    }
    // t.join();
    chunking.join();
}
