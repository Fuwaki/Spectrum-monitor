//! 频谱折线图
//! 使用 GPU 绘制实时频谱振幅曲线

use crate::pipeline::SpectrumLinePrimitive;
use crate::spectrum::SpectrumScene;
use iced::mouse;
use iced::widget::shader;
use iced::Rectangle;

/// 频谱折线图场景的包装器
/// 引用 SpectrumScene 获取数据，但使用不同的渲染 Primitive
pub struct SpectrumLineScene<'a> {
    pub scene: &'a SpectrumScene,
}

impl<'a, Message> shader::Program<Message> for SpectrumLineScene<'a> {
    type State = ();
    type Primitive = SpectrumLinePrimitive;

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        bounds: Rectangle,
    ) -> Self::Primitive {
        // 从 scene 获取最新的频谱数据
        let (spectrum_data, data_length, gain_factor, log_scale, scale_a, scale_b) = 
            self.scene.get_line_data();
        
        SpectrumLinePrimitive::new(
            spectrum_data,
            data_length,
            gain_factor,
            log_scale,
            scale_a,
            scale_b,
            bounds.width as u32,
            bounds.height as u32,
        )
    }
}
