//! 频谱显示场景
//! 实现 iced shader::Program trait

use crate::pipeline::SpectrumPrimitive;
use iced::mouse;
use iced::widget::shader;
use iced::Rectangle;

const MAX_BUFFER_SIZE: usize = 16384;

/// 频谱显示场景，存储待渲染的数据
#[derive(Clone)]
pub struct SpectrumScene {
    /// 频谱数据 (FFT结果的幅度)
    spectrum_data: Vec<f32>,
    /// 数据长度
    data_length: u32,
    /// 增益因子
    gain_factor: f32,
    /// 对数缩放参数 (0.0 = 线性, 1.0 = 全对数)
    log_scale: f32,
    /// 纹理高度 (= FFT大小 / 2)
    texture_height: u32,
    /// 数据版本号，每次有新数据时递增
    data_version: u64,
    /// Y轴缩放范围 (0.0-1.0)
    scale_a: f32,
    scale_b: f32,
    /// 是否暂停显示（但继续录音）
    paused: bool,
}

impl SpectrumScene {
    pub fn new(texture_height: u32) -> Self {
        Self {
            spectrum_data: vec![0.0; MAX_BUFFER_SIZE],
            data_length: 0,
            gain_factor: 0.15,
            log_scale: 0.5,
            texture_height,
            data_version: 0,
            scale_a: 0.0,
            scale_b: 1.0,
            paused: false,
        }
    }

    /// 推送新的频谱数据
    pub fn push_spectrum_data(&mut self, data: &[f32], gain: f32) {
        if self.paused {
            return; // 暂停时不更新数据
        }
        
        let len = data.len().min(MAX_BUFFER_SIZE);
        self.spectrum_data[..len].copy_from_slice(&data[..len]);
        self.data_length = len as u32;
        self.gain_factor = gain;
        self.data_version = self.data_version.wrapping_add(1);
    }

    /// 设置对数缩放
    pub fn set_log_scale(&mut self, scale: f32) {
        self.log_scale = scale.clamp(0.0, 1.0);
    }

    /// 设置Y轴缩放范围
    pub fn set_scale(&mut self, a: f32, b: f32) {
        self.scale_a = a.clamp(0.0, 1.0);
        self.scale_b = b.clamp(0.0, 1.0);
        // 确保 a < b
        if self.scale_a >= self.scale_b {
            self.scale_b = (self.scale_a + 0.01).min(1.0);
        }
    }

    /// 重置Y轴缩放到默认
    pub fn reset_scale(&mut self) {
        self.scale_a = 0.0;
        self.scale_b = 1.0;
    }

    /// 调整纹理高度（FFT大小改变时）
    pub fn resize_height(&mut self, new_height: u32) {
        self.texture_height = new_height;
    }

    /// 切换暂停状态
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    /// 是否暂停
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// 获取Y轴缩放范围
    pub fn get_scale(&self) -> (f32, f32) {
        (self.scale_a, self.scale_b)
    }

    /// 获取折线图绘制所需的数据
    /// 返回: (spectrum_data, data_length, gain_factor, log_scale, scale_a, scale_b)
    pub fn get_line_data(&self) -> (Vec<f32>, u32, f32, f32, f32, f32) {
        (
            self.spectrum_data.clone(),
            self.data_length,
            self.gain_factor,
            self.log_scale,
            self.scale_a,
            self.scale_b,
        )
    }
}

impl<Message> shader::Program<Message> for SpectrumScene {
    type State = ();
    type Primitive = SpectrumPrimitive;

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        bounds: Rectangle,
    ) -> Self::Primitive {
        SpectrumPrimitive::new(
            self.spectrum_data.clone(),
            self.data_length,
            self.gain_factor,
            self.log_scale,
            self.texture_height,
            bounds.width as u32,
            self.data_version,
            self.scale_a,
            self.scale_b,
        )
    }
}
