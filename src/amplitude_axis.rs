//! 强度轴组件
//! 显示振幅刻度 (水平轴)

use iced::widget::{canvas, Canvas};
use iced::{mouse, Color, Element, Fill, Length, Point, Rectangle, Renderer, Theme};

/// 强度轴组件
pub struct AmplitudeAxis {
    // 可以在此添加更多配置，如颜色、字体大小等
}

impl AmplitudeAxis {
    pub fn new() -> Self {
        Self {}
    }

    pub fn view(&self) -> Element<'static, crate::Message> {
        Canvas::new(AmplitudeAxisCanvas)
            .width(Fill)
            .height(Length::Fixed(30.0)) // 固定高度
            .into()
    }
}

struct AmplitudeAxisCanvas;

impl canvas::Program<crate::Message> for AmplitudeAxisCanvas {
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
        
        // 背景透明，不遮挡数据
        
        // 降低透明度让数据能透出来
        let text_color = Color::from_rgba(0.7, 0.7, 0.8, 0.8);
        let tick_color = Color::from_rgba(0.4, 0.4, 0.5, 0.5);
        let border_color = Color::from_rgba(0.3, 0.3, 0.4, 0.3);
        
        let width = bounds.width;

        let ticks_db: [(f32, &str); 9] = [
            (0.0, "-60"),      // -60dB -> 0.0
            (0.125, "-50"),    // -50dB -> 0.125
            (0.25, "-40"),     // -40dB -> 0.25
            (0.375, "-30"),    // -30dB -> 0.375
            (0.5, "-20"),      // -20dB -> 0.5
            (0.625, "-10"),    // -10dB -> 0.625
            (0.75, "0"),       // 0dB -> 0.75
            (0.875, "+10"),    // +10dB -> 0.875
            (1.0, "+20"),      // +20dB -> 1.0
        ];
        
        // 上边框线
        let border_path = canvas::Path::line(
            Point::new(0.0, 1.0),
            Point::new(width, 1.0),
        );
        frame.stroke(
            &border_path,
            canvas::Stroke::default()
                .with_color(border_color)
                .with_width(1.0),
        );

        for &(pos, label) in &ticks_db {
            let x = pos * width;
            
            // 刻度线
            let tick_path = canvas::Path::line(
                Point::new(x, 0.0),
                Point::new(x, 6.0),
            );
            frame.stroke(
                &tick_path,
                canvas::Stroke::default()
                    .with_color(tick_color)
                    .with_width(1.0),
            );
            
            // 刻度文字
            let text_width = 25.0;
            let text_x = if pos < 0.01 {
                x + 2.0
            } else if pos > 0.99 {
                x - text_width
            } else {
                x - text_width / 2.0
            };
            
            let text = canvas::Text {
                content: format!("{}dB", label),
                position: Point::new(text_x, 8.0),
                color: text_color,
                size: iced::Pixels(11.0),
                ..Default::default()
            };
            frame.fill_text(text);
        }
        
        vec![frame.into_geometry()]
    }
}
