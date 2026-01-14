use std::result::Result::Ok;
use rustfft::num_complex::ComplexFloat;
use std::sync::mpsc;

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Stream,
};

pub struct Audio {
    stream: Option<Stream>,
    rx: mpsc::Receiver<Vec<f32>>,
    tx: mpsc::Sender<Vec<f32>>,
    fftsize: usize,
    fftwindow: FFTWindow,
    buffer: Vec<f32>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FFTWindow {
    Rectangular,
    Hanning,
    Hamming,
    Blackman,
}

impl std::fmt::Display for FFTWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                FFTWindow::Rectangular => "矩形窗",

                FFTWindow::Hanning => "汉宁窗",
                FFTWindow::Hamming => "汉明窗",
                FFTWindow::Blackman => "布莱克曼窗",
            }
        )
    }
}
impl Audio {
    fn create_stream(tx: mpsc::Sender<Vec<f32>>) -> Result<Stream, anyhow::Error> {
        let host = cpal::default_host();
        let device = host.default_input_device().expect("找不到默认输入设备");
        let config = device.default_input_config()?;
        // let config = SupportedStreamConfig::new(
        //     default_config.channels(),
        //     SampleRate(44100), //TODO: 这里可以以后设计成可变的 但是分析了一下其实意义不大 采样率的提高不会带来频谱精度的提升 只会带来频率广度的提升 而超声波一般来说设备没有记录
        //     *default_config.buffer_size(),
        //     cpal::SampleFormat::F32,
        // );
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
            buffer: Vec::new(),
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
    pub fn fetch_data(&mut self) -> Option<(Vec<f32>, usize)> {
        // 1. 将所有新消息读入缓冲区
        while let Ok(mut msg) = self.rx.try_recv() {
            self.buffer.extend(msg.drain(..));
        }

        // 2. 如果数据不足一个FFT窗口，返回None
        if self.buffer.len() < self.fftsize {
            return None;
        }

        // 3. 为了低延迟，我们只取最新的 fftsize 个数据
        // 如果缓冲区积累了太多数据（比如因为处理变慢），直接丢弃旧数据
        let start_index = self.buffer.len().saturating_sub(self.fftsize);
        
        // 4. 获取用于FFT的数据切片（不需要 drain，保留 buffer 给下一次 overlap ?）
        // 实际上为了简单和性能，如果我们要 overlap，可以在 buffer 头部保留一些数据
        // 但这里我们简单点：总是取最新的，然后清空 buffer 到只剩 overlap 部分
        
        let window_data = self.buffer[start_index..].to_vec();
        
        // 5. 保留一部分数据用于下次重叠 (比如保留最新的 75%)
        // 这样下次有了新数据后，又能组成一个新的窗口
        let overlap_size = self.fftsize * 3 / 4;
        let new_start = self.buffer.len().saturating_sub(overlap_size);
        
        // 创建新缓冲区，只包含重叠部分
        // (比 drain 效率低一点点但逻辑清晰，buffer 一般不大)
        // 使用 drain 更好：删除前面不需要的数据
        if new_start > 0 {
            self.buffer.drain(0..new_start);
        }
        
        return Some((self.do_fft(window_data), self.buffer.len()));
    }

    pub fn stop(&mut self) -> () {
        self.stream.as_ref().unwrap().pause().unwrap();
    }
}
