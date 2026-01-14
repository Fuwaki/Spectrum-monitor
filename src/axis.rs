//! 频率轴组件
//! 显示频率刻度，支持对数缩放

use iced::widget::{canvas, Canvas};
use iced::{mouse, Color, Element, Fill, Length, Point, Rectangle, Renderer, Size, Theme};

/// 坐标轴组件
pub struct Axis {
    /// 是否显示网格线
    show_grid: bool,
}

impl Axis {
    pub fn new() -> Self {
        Self { show_grid: true }
    }

    /// 创建一个带参数的视图
    /// - min_freq: 最小频率 (通常是0)
    /// - max_freq: 最大频率 (采样率/2)
    /// - scale_a: Y轴缩放起点 (0.0-1.0)
    /// - scale_b: Y轴缩放终点 (0.0-1.0)
    /// - log_scale: 对数缩放参数
    pub fn view(
        &self,
        min_freq: f32,
        max_freq: f32,
        scale_a: f32,
        scale_b: f32,
        log_scale: f32,
    ) -> Element<'static, super::Message> {
        Canvas::new(AxisCanvas {
            min_freq,
            max_freq,
            scale_a,
            scale_b,
            show_grid: self.show_grid,
            log_scale,
        })
        .width(Length::Fixed(60.0))
        .height(Fill)
        .into()
    }
}

struct AxisCanvas {
    min_freq: f32,
    max_freq: f32,
    scale_a: f32,
    scale_b: f32,
    show_grid: bool,
    log_scale: f32,
}

impl canvas::Program<super::Message> for AxisCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        
        let height = bounds.height;
        let width = bounds.width;
        
        // 背景
        frame.fill_rectangle(
            Point::ORIGIN,
            Size::new(width, height),
            Color::from_rgb(0.08, 0.08, 0.12),
        );
        
        // 生成刻度
        let ticks = generate_frequency_ticks(self.min_freq, self.max_freq);
        
        // 文字样式
        let text_color = Color::from_rgb(0.7, 0.7, 0.8);
        let tick_color = Color::from_rgb(0.4, 0.4, 0.5);
        
        let global_range = self.max_freq - self.min_freq;

        for (freq, label) in ticks {
            let freq_norm = (freq - self.min_freq) / global_range;
            
            let mut y_global_norm = freq_norm;
            
            if self.log_scale > 0.0 {
                let base = 1.0 + self.log_scale * 100.0;
                y_global_norm = (freq_norm * (base - 1.0) + 1.0).ln() / base.ln();
            }
            
            let visible_y = (y_global_norm - self.scale_a) / (self.scale_b - self.scale_a);
            let screen_y = height * (1.0 - visible_y);
            
            if screen_y < -10.0 || screen_y > height + 10.0 {
                continue;
            }
            
            // 画刻度线
            let tick_path = canvas::Path::line(
                Point::new(width - 8.0, screen_y),
                Point::new(width, screen_y),
            );
            frame.stroke(
                &tick_path,
                canvas::Stroke::default()
                    .with_color(tick_color)
                    .with_width(1.0),
            );
            
            // 画刻度文字
            let text = canvas::Text {
                content: label,
                position: Point::new(4.0, screen_y - 7.0),
                color: text_color,
                size: iced::Pixels(11.0),
                ..Default::default()
            };
            frame.fill_text(text);
        }
        
        // 右边界线
        let border_path = canvas::Path::line(
            Point::new(width - 1.0, 0.0),
            Point::new(width - 1.0, height),
        );
        frame.stroke(
            &border_path,
            canvas::Stroke::default()
                .with_color(Color::from_rgb(0.3, 0.3, 0.4))
                .with_width(1.0),
        );
        
        vec![frame.into_geometry()]
    }
}

fn generate_frequency_ticks(min_freq: f32, max_freq: f32) -> Vec<(f32, String)> {
    let mut ticks = Vec::new();
    let levels = [
        (100.0, 1000.0, 100.0),
        (1000.0, 5000.0, 500.0),
        (5000.0, 10000.0, 1000.0),
        (10000.0, 22050.0, 2000.0),
    ];
    if min_freq < 100.0 {
        let mut f = 20.0;
        while f < 100.0 {
            if f >= min_freq { ticks.push((f, format!("{:.0}", f))); }
            f += 20.0;
        }
    }
    for (start, end, step) in levels {
        let mut freq = start;
        while freq < end && freq <= max_freq {
            if freq >= min_freq {
                let label = if freq >= 1000.0 {
                    format!("{:.1}k", freq / 1000.0)
                } else {
                    format!("{:.0}", freq)
                };
                ticks.push((freq, label));
            }
            freq += step;
        }
    }
    ticks
}
