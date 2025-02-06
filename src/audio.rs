use std::result::Result::Ok;
use rustfft::num_complex::ComplexFloat;
use std::sync::mpsc;

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleRate, Stream, SupportedStreamConfig,
};

pub struct Audio {
    stream: Option<Stream>,
    rx: mpsc::Receiver<Vec<f32>>,
    tx: mpsc::Sender<Vec<f32>>,
    fftsize: usize,
    fftwindow: FFTWindow,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FFTWindow {
    Rectangular,
    Hanning,
    Hamming,
    Blackman,
}
impl Audio {
    fn create_stream(tx: mpsc::Sender<Vec<f32>>) -> Result<Stream, anyhow::Error> {
        let host = cpal::default_host();
        let device = host.default_input_device().expect("找不到默认输入设备");
        let default_config = device.default_input_config()?;
        let config = SupportedStreamConfig::new(
            default_config.channels(),
            SampleRate(44100), //TODO: 这里可以以后设计成可变的 但是分析了一下其实意义不大 采样率的提高不会带来频谱精度的提升 只会带来频率广度的提升 而超声波一般来说设备没有记录
            *default_config.buffer_size(),
            cpal::SampleFormat::F32,
        );
        let err_fn = move |err| {
            panic!("an error occurred on stream: {}", err);
        };

        let channels = config.channels();
        let stream = device.build_input_stream(
            &config.into(),
            move |data: &[f32], _| {
                //声道转换
                let mut mono_data = Vec::with_capacity(data.len() / channels as usize);
                for i in (0..data.len()).step_by(channels as usize) {
                    mono_data.push((data[i] + data[i + 1]) / channels as f32);
                }
                tx.send(mono_data).unwrap()
            },
            err_fn,
            None,
        )?;
        stream.play()?;
        Ok(stream)
    }
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<Vec<f32>>();
        Self {
            stream: None,
            rx,
            tx,
            fftsize: 1024,
            fftwindow: FFTWindow::Hanning,
        }
    }
    pub fn start(&mut self)->Result<(),anyhow::Error> {
        self.stream = Some(Audio::create_stream(self.tx.clone())?);
        Ok(())
    }
    fn fft_window(pcm_data: &mut Vec<f32>, window_func: FFTWindow) {
        let len = pcm_data.len();
        match window_func {
            FFTWindow::Rectangular => {
                // Rectangular window: no changes to pcm_data
            }
            FFTWindow::Hanning => {
                for n in 0..len {
                    let multiplier = 0.5
                        * (1.0
                            - (2.0 * std::f32::consts::PI * n as f32 / (len as f32 - 1.0)).cos());
                    pcm_data[n] *= multiplier;
                }
            }
            FFTWindow::Hamming => {
                for n in 0..len {
                    let multiplier = 0.54
                        - 0.46 * (2.0 * std::f32::consts::PI * n as f32 / (len as f32 - 1.0)).cos();
                    pcm_data[n] *= multiplier;
                }
            }
            FFTWindow::Blackman => {
                for n in 0..len {
                    let multiplier = 0.42
                        - 0.5 * (2.0 * std::f32::consts::PI * n as f32 / (len as f32 - 1.0)).cos()
                        + 0.08 * (4.0 * std::f32::consts::PI * n as f32 / (len as f32 - 1.0)).cos();
                    pcm_data[n] *= multiplier;
                }
            }
        }
    }

    pub fn set_fft_window_func(&mut self, window_func: FFTWindow) {
        self.fftwindow = window_func
    }
    pub fn set_fft_size(&mut self, fftsize: usize) {
        self.fftsize = fftsize
    }

    fn do_fft(&self, mut pcm_data: Vec<f32>) -> Vec<f32> {
        // Perform a forward FFT of size 1234
        use rustfft::{num_complex::Complex, FftPlanner};
        Audio::fft_window(&mut pcm_data, self.fftwindow);
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(pcm_data.len());

        // let mut buffer = vec![Complex { re: 0.0, im: 0.0 }; 1234];
        let buffer = pcm_data
            .into_iter()
            .map(|item| Complex { re: item, im: 0.0 });
        let mut b = Vec::from_iter(buffer);
        // println!("{:?}",b);

        fft.process(&mut b);
        // println!("{:+.2}", b[0].re);
        let effective = &b[0..b.len() / 2 + 1];
        let magnitudes: Vec<f32> = effective.iter().map(|item| item.abs() * 2.0).collect(); //乘以2 因为我们取的是单边 作补偿
                                                                                            // println!("结果{:?}",magnitudes);
        magnitudes
    }
    pub fn fetch_data(&self) -> Option<(Vec<f32>, usize)> {
        static TEMP: std::sync::RwLock<Vec<f32>> = std::sync::RwLock::new(Vec::new());

        let mut temp = TEMP.write().unwrap();
        while let Ok(mut msg) = self.rx.try_recv() {
            temp.extend(msg.drain(..));
        }

        if temp.len() > self.fftsize {
            let a: Vec<f32> = temp.drain(0..self.fftsize).collect();
            return Some((self.do_fft(a), temp.len()));
        } else {
            return None;
        }
    }

    pub fn stop(&mut self) -> () {
        self.stream.as_ref().unwrap().pause().unwrap();
    }
}
