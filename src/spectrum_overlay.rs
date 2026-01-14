//! 频谱及折线图的鼠标辅助层
//! 显示鼠标位置对应的频率和振幅，以及主峰标识

use iced::widget::canvas;
use iced::{mouse, Color, Point, Rectangle, Renderer, Theme};

pub struct SpectrumOverlay {
    pub data: Vec<f32>,
    pub data_length: u32,
    pub scale_a: f32,
    pub scale_b: f32,
    pub log_scale: f32,
    pub gain: f32,
    pub max_freq: f32,
}

/// 将原始振幅转换为归一化的显示值 (使用对数/dB刻度)
/// 与 line.wgsl 中的 amplitude_to_normalized 保持一致
fn amplitude_to_normalized(value: f32, gain: f32) -> f32 {
    let ref_value = 1.0 / gain;
    let ratio = value / ref_value;
    
    if ratio <= 0.001 {
        return 0.0;
    }
    
    // dB = 20 * log10(ratio)
    let db = 20.0 * (ratio).log10();
    
    // 映射 dB 到 0-1: -60dB -> 0, 0dB -> 0.75, +20dB -> 1.0
    let normalized = (db + 60.0) / 80.0;
    normalized.clamp(0.0, 1.0)
}

/// 将归一化值转换回 dB
fn normalized_to_db(normalized: f32) -> f32 {
    normalized * 80.0 - 60.0
}

/// 对数缩放的逆变换 (log_space -> freq_normalized)
fn inverse_log_scale(log_space_y: f32, log_scale: f32) -> f32 {
    if log_scale <= 0.0 {
        log_space_y
    } else {
        let s = log_scale;
        let base = 1.0 + s * 100.0;
        (base.powf(log_space_y) - 1.0) / (base - 1.0)
    }
}

/// 对数缩放的正向变换 (freq_normalized -> log_space)
fn apply_log_scale(freq: f32, log_scale: f32) -> f32 {
    if log_scale <= 0.0 {
        freq
    } else {
        let s = log_scale;
        let base = 1.0 + s * 100.0;
        (freq * (base - 1.0) + 1.0).ln() / base.ln()
    }
}

/// 峰值信息
struct Peak {
    _freq_index: usize,
    freq_hz: f32,
    amplitude: f32,
    screen_y: f32,
}

impl canvas::Program<crate::Message> for SpectrumOverlay {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        let data_len = self.data_length as usize;

        // ========== 主峰自动标识（始终显示） ==========
        if data_len > 0 {
            let peaks = self.find_peaks(bounds.height);

            for peak in peaks.iter().take(5) {
                let peak_x = amplitude_to_normalized(peak.amplitude, self.gain) * bounds.width;

                let peak_pos = Point::new(peak_x, peak.screen_y);
                let peak_circle = canvas::Path::circle(peak_pos, 5.0);
                frame.fill(&peak_circle, Color::from_rgba(0.2, 1.0, 0.4, 0.8));

                let peak_db = normalized_to_db(amplitude_to_normalized(peak.amplitude, self.gain));
                let peak_label = format!("{:.0}Hz\n{:.0}dB", peak.freq_hz, peak_db);

                let label_x = if peak_x + 60.0 > bounds.width {
                    peak_x - 55.0
                } else {
                    peak_x + 8.0
                };
                let label_y = if peak.screen_y < 30.0 {
                    peak.screen_y + 5.0
                } else {
                    peak.screen_y - 25.0
                };

                let peak_text = canvas::Text {
                    content: peak_label,
                    position: Point::new(label_x, label_y),
                    color: Color::from_rgba(0.4, 1.0, 0.6, 0.9),
                    size: iced::Pixels(11.0),
                    ..Default::default()
                };
                frame.fill_text(peak_text);
            }
        }

        // ========== 鼠标读数（仅鼠标在图内时显示） ==========
        if let Some(cursor_position) = cursor.position_in(bounds) {
            let y = cursor_position.y;
            let height = bounds.height;

            // 水平辅助线
            let guide_color = Color::from_rgba(1.0, 1.0, 1.0, 0.3);
            let guide_path = canvas::Path::line(Point::new(0.0, y), Point::new(bounds.width, y));
            frame.stroke(
                &guide_path,
                canvas::Stroke {
                    style: canvas::Style::Solid(guide_color),
                    width: 1.0,
                    line_dash: canvas::LineDash {
                        segments: &[4.0, 4.0],
                        offset: 0,
                    },
                    ..canvas::Stroke::default()
                },
            );

            // 屏幕 y -> freq_normalized
            let visible_y = 1.0 - (y / height).clamp(0.0, 1.0);
            let log_space_y = visible_y * (self.scale_b - self.scale_a) + self.scale_a;
            let freq_normalized = inverse_log_scale(log_space_y, self.log_scale).clamp(0.0, 1.0);
            let freq_hz = freq_normalized * self.max_freq;

            // 取该频率 bin 的幅值
            let value = if data_len > 0 {
                let max_index = (data_len - 1).max(1);
                let index = (freq_normalized * max_index as f32).round() as usize;
                if index < data_len && index < self.data.len() {
                    self.data[index]
                } else {
                    0.0
                }
            } else {
                0.0
            };

            let amplitude_visual = amplitude_to_normalized(value, self.gain);
            let amplitude_db = normalized_to_db(amplitude_visual);

            // 红点（只画点，不画垂直辅助线）
            let point_x = amplitude_visual * bounds.width;
            let point_pos = Point::new(point_x, y);
            let point_path = canvas::Path::circle(point_pos, 4.0);
            frame.fill(&point_path, Color::from_rgb(1.0, 0.2, 0.2));

            // 文本
            let info_text = format!("{:.1} Hz\n{:.1} dB", freq_hz, amplitude_db);

            let mut text_pos = Point::new(cursor_position.x + 10.0, y - 20.0);
            if text_pos.x + 100.0 > bounds.width {
                text_pos.x = cursor_position.x - 110.0;
            }
            if text_pos.y < 0.0 {
                text_pos.y = y + 10.0;
            }

            let text = canvas::Text {
                content: info_text,
                position: text_pos,
                color: Color::WHITE,
                size: iced::Pixels(14.0),
                ..Default::default()
            };
            frame.fill_text(text);
        }

        vec![frame.into_geometry()]
    }
}

impl SpectrumOverlay {
    /// 在可见范围内找出主要峰值
    fn find_peaks(&self, view_height: f32) -> Vec<Peak> {
        let data_len = self.data_length as usize;
        if data_len < 3 {
            return vec![];
        }
        
        let mut peaks = Vec::new();
        let max_index = (data_len - 1).max(1);
        
        // 计算可见范围对应的数据索引
        // scale_a/scale_b 是 log_space 坐标
        let freq_start = inverse_log_scale(self.scale_a, self.log_scale).clamp(0.0, 1.0);
        let freq_end = inverse_log_scale(self.scale_b, self.log_scale).clamp(0.0, 1.0);
        
        let start_idx = (freq_start * max_index as f32) as usize;
        let end_idx = ((freq_end * max_index as f32) as usize).min(data_len - 1);
        
        // 简单的峰值检测：比相邻点都大，且超过一定阈值
        let threshold = 0.1 / self.gain; // 动态阈值
        
        for i in (start_idx + 1)..(end_idx.saturating_sub(1)) {
            if i >= self.data.len() { break; }
            
            let val = self.data[i];
            let prev = self.data[i - 1];
            let next = self.data[i + 1];
            
            // 是否是局部最大值
            if val > prev && val > next && val > threshold {
                // 计算屏幕位置
                let freq_normalized = i as f32 / max_index as f32;
                let log_space_y = apply_log_scale(freq_normalized, self.log_scale);
                let visible_y = (log_space_y - self.scale_a) / (self.scale_b - self.scale_a);
                let screen_y = view_height * (1.0 - visible_y);
                
                peaks.push(Peak {
                    _freq_index: i,
                    freq_hz: freq_normalized * self.max_freq,
                    amplitude: val,
                    screen_y,
                });
            }
        }
        
        // 按振幅排序，取最大的几个
        peaks.sort_by(|a, b| b.amplitude.partial_cmp(&a.amplitude).unwrap_or(std::cmp::Ordering::Equal));
        
        peaks
    }
}
