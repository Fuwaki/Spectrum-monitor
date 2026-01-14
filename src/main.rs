#![windows_subsystem = "windows"]

mod audio;
mod pipeline;
mod spectrum;
mod axis;
mod spectrum_line;

use audio::{Audio, FFTWindow};
use spectrum::SpectrumScene;
use axis::Axis;
use spectrum_line::SpectrumLineScene;

use iced::mouse;
use iced::widget::{
    button, column, container, horizontal_space, mouse_area, pick_list, row, slider, stack, text, tooltip,
};
use iced::window;
use iced::{Center, Color, Element, Fill, Font, Subscription, Task, Theme};
use iced::time::{self, Duration, Instant};
use iced::keyboard;
use iced::event::{self, Event};

// 嵌入中文字体
const MISANS_FONT_BYTES: &[u8] = include_bytes!("../MiSans-Medium.ttf");

// 定义字体常量
const MISANS: Font = Font {
    family: iced::font::Family::Name("MiSans"),
    weight: iced::font::Weight::Medium,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

fn main() -> iced::Result {
    env_logger::init();

    iced::application("频谱监视器", App::update, App::view)
        .subscription(App::subscription)
        .theme(|_| Theme::TokyoNightStorm)
        .font(MISANS_FONT_BYTES)
        .default_font(MISANS)
        .window(window::Settings {
            size: iced::Size::new(1200.0, 700.0),
            ..Default::default()
        })
        .antialiasing(true)
        .run_with(App::new)
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
    ToggleAudio,
    TogglePause,
    FFTSizeChanged(u32),
    GainChanged(f32),
    LogScaleChanged(f32),
    WindowFuncChanged(FFTWindow),
    // 鼠标交互
    MouseInteraction(mouse::Event),
    // 键盘快捷键
    KeyPressed(keyboard::Key),
    // 重置缩放
    ResetScale,
    // 分隔条拖拽
    SplitDragStart,
    SplitDragEnd,
    SplitDrag(iced::Point),
}

struct App {
    audio: Audio,
    audio_running: bool,
    scene: SpectrumScene,
    // 频率轴辅助组件
    axis: Axis,
    // 是否正在拖拽（频谱缩放）
    is_dragging: bool,
    last_mouse_pos: Option<iced::Point>,
    
    // 分隔条拖拽状态
    is_split_dragging: bool,
    // 瀑布图宽度比例 (0.0 - 1.0)
    spectrum_ratio: f32,
    
    fft_size: u32,
    gain: f32,
    log_scale: f32,
    window_func: FFTWindow,
    error_msg: Option<String>,
    // Y轴缩放参数
    scale_a: f32,
    scale_b: f32,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                audio: Audio::new(),
                audio_running: false,
                scene: SpectrumScene::new(512), // fft_size / 2
                axis: Axis::new(),
                is_dragging: false,
                last_mouse_pos: None,
                is_split_dragging: false,
                spectrum_ratio: 0.7, // 瀑布图占 70%
                fft_size: 1024,
                gain: 0.15,
                log_scale: 0.5,
                window_func: FFTWindow::Hanning,
                error_msg: None,
                scale_a: 0.0,
                scale_b: 1.0,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick(_now) => {
                // 获取音频数据并更新场景
                if self.audio_running {
                    while let Some((data, _remain)) = self.audio.fetch_data() {
                        self.scene.push_spectrum_data(&data, self.gain);
                    }
                }
            }
            Message::ToggleAudio => {
                if self.audio_running {
                    self.audio.stop();
                    self.audio_running = false;
                } else {
                    match self.audio.start() {
                        Ok(()) => {
                            self.audio_running = true;
                            self.error_msg = None;
                        }
                        Err(e) => {
                            self.error_msg = Some(format!("音频启动失败: {}", e));
                        }
                    }
                }
            }
            Message::TogglePause => {
                self.scene.toggle_pause();
            }
            Message::FFTSizeChanged(size) => {
                // 确保是2的幂
                let power = (size as f32).log2().round() as u32;
                self.fft_size = 2u32.pow(power.clamp(8, 14)); // 256 到 16384
                self.audio.set_fft_size(self.fft_size as usize);
                self.scene.resize_height(self.fft_size / 2);
            }
            Message::GainChanged(gain) => {
                self.gain = gain;
            }
            Message::LogScaleChanged(scale) => {
                self.log_scale = scale;
                self.scene.set_log_scale(scale);
            }
            Message::WindowFuncChanged(func) => {
                self.window_func = func;
                self.audio.set_fft_window_func(func);
            }
            Message::MouseInteraction(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if !self.is_split_dragging {
                    self.is_dragging = true;
                    self.last_mouse_pos = None; // Reset to catch next move
                }
            }
            Message::MouseInteraction(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                self.is_dragging = false;
                self.is_split_dragging = false;
                self.last_mouse_pos = None;
            }
            Message::MouseInteraction(mouse::Event::CursorMoved { position }) => {
                if self.is_dragging {
                    if let Some(last_pos) = self.last_mouse_pos {
                        let dy = position.y - last_pos.y;
                        
                        // 拖拽平移
                        // 移动距离相对于总高度的比例
                        let sensitivity = 0.002;
                        let move_delta = dy * sensitivity * (self.scale_b - self.scale_a);
                        
                        // 向上拖动 (dy < 0) 应该查看更上方的内容 (scale 减小？)
                        // 屏幕坐标 Y 向下增加。
                        // scale_a (Bot) -> scale_b (Top)
                        // 如果 Viewport 往上移，意味着 a 和 b 都增加
                        // 鼠标向下移 (dy > 0), 视口应该向上移 (内容向下)
                        
                        let new_a = self.scale_a + move_delta;
                        let new_b = self.scale_b + move_delta;
                        
                        // 简单的边界检查 (允许稍微拖出一点，但主要保持范围)
                        if new_a >= -0.1 && new_b <= 1.1 {
                            self.scale_a = new_a.clamp(0.0, 1.0);
                            let range = self.scale_b - self.scale_a; // keep old range to avoid drifting size
                            self.scale_b = (self.scale_a + range).clamp(0.0, 1.0);
                             // Refresh range just in case clamp changed size
                             let final_range = self.scale_b - self.scale_a;
                             if final_range < range - 0.001 {
                                 // Hitting top
                                  self.scale_a = self.scale_b - range;
                             }

                            self.scene.set_scale(self.scale_a, self.scale_b);
                        }
                    }
                    self.last_mouse_pos = Some(position);
                } else {
                    // Update mouse position for zoom centering (not strictly needed if we use Cursor directly in Wheel)
                     self.last_mouse_pos = Some(position);
                }
            }
            Message::MouseInteraction(mouse::Event::WheelScrolled { delta }) => {
                // 鼠标滚轮缩放Y轴 - 以光标位置为中心
                const TOUCHPAD_SENSITIVITY: f32 = 0.002;
                const MOUSEWHEEL_SENSITIVITY: f32 = 0.1;

                let d = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y * MOUSEWHEEL_SENSITIVITY,
                    mouse::ScrollDelta::Pixels { y, .. } => y * TOUCHPAD_SENSITIVITY,
                };
                
                if d == 0.0 { return Task::none(); }

                // Determine zoom center based on mouse position
                // We need to know where the mouse is relative to the element height.
                // Since we don't have element bounds here easily without more plumbing,
                // let's assume center zoom or try to get approximate relative pos.
                // For now, center zoom is safer/simpler or we can use 0.5 as middle.
                // To do mouse-relative zoom, we'd need to know the widget geometry.
                // Let's stick to center-based for now or improved center.
                // Better: Use `last_mouse_pos` if available, but we need widget bounds.
                // Let's assume the user is focusing on the middle of the current view roughly.
                
                let range = self.scale_b - self.scale_a;
                
                // If we want to zoom in (d > 0), range decreases.
                let new_range = (range * (1.0 - d * 0.5)).clamp(0.001, 1.0);
                
                // Calculate center (could be customizable)
                let center = (self.scale_a + self.scale_b) / 2.0;

                let half_range = new_range / 2.0;
                let mut new_a = center - half_range;
                let mut new_b = center + half_range;
                
                // Clamp to [0, 1] but try to maintain range size
                if new_a < 0.0 {
                    new_a = 0.0;
                    new_b = new_range;
                } else if new_b > 1.0 {
                    new_b = 1.0;
                    new_a = 1.0 - new_range;
                }
                
                self.scale_a = new_a.clamp(0.0, 1.0);
                self.scale_b = new_b.clamp(0.0, 1.0);
                
                self.scene.set_scale(self.scale_a, self.scale_b);
            }
            Message::SplitDragStart => {
                self.is_split_dragging = true;
            }
            Message::SplitDragEnd => {
                self.is_split_dragging = false;
            }
            Message::SplitDrag(position) => {
                if self.is_split_dragging {
                    // 假设频率轴宽度约为 50px，计算有效区域
                    // position.x 是相对于整个窗口的位置
                    // 我们需要计算相对于图表区域的比例
                    // 粗略估计：窗口宽度减去边距和频率轴
                    let axis_width = 50.0;
                    let padding = 20.0; // 左右边距
                    let available_width = 1200.0 - axis_width - padding * 2.0; // 假设窗口宽度
                    let relative_x = position.x - axis_width - padding;
                    
                    if available_width > 0.0 && relative_x > 0.0 {
                        let ratio = (relative_x / available_width).clamp(0.2, 0.9);
                        self.spectrum_ratio = ratio;
                    }
                }
            }
            _ => {}
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        // 1. 折线图 (Line Plot)
        let line_scene = SpectrumLineScene { scene: &self.scene };
        let line_plot = iced::widget::shader(line_scene)
            .width(Fill)
            .height(Fill);
            
        // 2. 瀑布图 (Spectrogram)
        let spectrum_shader = iced::widget::shader(&self.scene)
            .width(Fill)
            .height(Fill);
        
        // 可拖拽的分隔条
        let splitter = mouse_area(
            container(horizontal_space())
                .width(6)
                .height(Fill)
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(
                        if self.is_split_dragging {
                            Color::from_rgb(0.4, 0.6, 0.9)
                        } else {
                            Color::from_rgb(0.3, 0.3, 0.4)
                        }
                    )),
                    ..Default::default()
                })
        )
        .on_press(Message::SplitDragStart)
        .on_release(Message::SplitDragEnd)
        .interaction(mouse::Interaction::ResizingHorizontally);

        // 计算宽度比例
        let spectrum_width = iced::Length::FillPortion((self.spectrum_ratio * 100.0) as u16);
        let line_width = iced::Length::FillPortion(((1.0 - self.spectrum_ratio) * 100.0) as u16);
        
        // 交互区域覆盖整个图形部分
        let spectrum_area = mouse_area(
            row![
                // 频率轴 (Axis)
                 self.axis.view(
                    20.0, 
                    22050.0, 
                    self.scale_a, 
                    self.scale_b,
                    self.log_scale
                ),
                
                // Spectrogram - 左边，可调整宽度
                container(spectrum_shader)
                    .width(spectrum_width)
                    .style(|_| container::Style {
                        border: iced::Border {
                            color: Color::from_rgb(0.2, 0.2, 0.2),
                            width: 1.0,
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                
                // 可拖拽分隔条
                splitter,
                
                // Line Plot - 右边，可调整宽度
                container(line_plot)
                    .width(line_width)
                    .style(|_| container::Style {
                        border: iced::Border {
                            color: Color::from_rgb(0.2, 0.2, 0.2),
                            width: 1.0,
                            ..Default::default()
                        },
                         ..Default::default()
                    }),
            ]
            .spacing(0)
        )
        .on_press(Message::MouseInteraction(mouse::Event::ButtonPressed(mouse::Button::Left)))
        .on_release(Message::MouseInteraction(mouse::Event::ButtonReleased(mouse::Button::Left)))
        .on_move(|p| {
            if self.is_split_dragging {
                Message::SplitDrag(p)
            } else {
                Message::MouseInteraction(mouse::Event::CursorMoved { position: p })
            }
        })
        .on_scroll(|d| Message::MouseInteraction(mouse::Event::WheelScrolled { delta: d }));

        // 暂停指示器
        let pause_overlay = if self.scene.is_paused() {
            Some(
                container(
                    text("⏸ 已暂停 (按空格继续)")
                        .size(16)
                        .style(|_| text::Style { 
                            color: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.8)) 
                        })
                )
                .padding([8, 16])
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.6))),
                    border: iced::Border {
                        radius: 8.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
            )
        } else {
            None
        };

        let spectrum_content = if let Some(overlay) = pause_overlay {
            container(
                stack![
                    spectrum_area,
                    container(overlay)
                        .width(Fill)
                        .height(Fill)
                        .align_x(Center)
                        .align_y(Center)
                ]
            )
            .width(Fill)
            .height(Fill)
            .padding(5)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.08, 0.08, 0.12))),
                border: iced::Border {
                    color: Color::from_rgb(0.2, 0.2, 0.3),
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            })
        } else {
            container(spectrum_area)
                .width(Fill)
                .height(Fill)
                .padding(5)
                .style(|_theme| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgb(0.08, 0.08, 0.12))),
                    border: iced::Border {
                        color: Color::from_rgb(0.2, 0.2, 0.3),
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                })
        };

        // 控制面板
        let controls = self.view_controls();

        // 主布局
        column![
            spectrum_content,
            container(controls)
                .padding(15)
                .style(|_theme| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgba(0.1, 0.1, 0.15, 0.95))),
                    border: iced::Border {
                        color: Color::from_rgb(0.2, 0.2, 0.3),
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                }),
        ]
        .spacing(5)
        .padding(10)
        .into()
    }

    fn view_controls(&self) -> Element<'_, Message> {
        // 开始/停止录音按钮
        let toggle_btn = tooltip(
            button(
                text(if self.audio_running { "⏹ 停止" } else { "▶ 开始" })
                    .size(15)
            )
            .padding([6, 16])
            .on_press(Message::ToggleAudio)
            .style(if self.audio_running {
                button::danger
            } else {
                button::success
            }),
            "快捷键: S",
            tooltip::Position::Top,
        );

        // 暂停/继续显示按钮
        let pause_btn = tooltip(
            button(
                text(if self.scene.is_paused() { "▶ 继续" } else { "⏸ 暂停" })
                    .size(15)
            )
            .padding([6, 16])
            .on_press(Message::TogglePause)
            .style(if self.scene.is_paused() {
                button::primary
            } else {
                button::secondary
            }),
            "快捷键: 空格",
            tooltip::Position::Top,
        );

        // 重置缩放按钮
        let reset_btn = tooltip(
            button(text("↺ 重置").size(15))
                .padding([6, 12])
                .on_press(Message::ResetScale)
                .style(button::secondary),
            "快捷键: R",
            tooltip::Position::Top,
        );

        // FFT大小控制
        let fft_size_control = row![
            text("FFT").size(13).style(|_| text::Style { color: Some(Color::from_rgb(0.6, 0.6, 0.7)) }),
            slider(8.0..=14.0, (self.fft_size as f32).log2(), |v| {
                Message::FFTSizeChanged(2u32.pow(v as u32))
            })
            .step(1.0)
            .width(100),
            text(format!("{}", self.fft_size)).size(13).width(45),
        ]
        .spacing(6)
        .align_y(Center);

        // 增益控制
        let gain_control = row![
            text("增益").size(13).style(|_| text::Style { color: Some(Color::from_rgb(0.6, 0.6, 0.7)) }),
            slider(0.01..=1.0, self.gain, Message::GainChanged)
                .step(0.01)
                .width(100),
            text(format!("{:.2}", self.gain)).size(13).width(35),
        ]
        .spacing(6)
        .align_y(Center);

        // 对数缩放控制
        let log_scale_control = row![
            text("对数").size(13).style(|_| text::Style { color: Some(Color::from_rgb(0.6, 0.6, 0.7)) }),
            slider(0.0..=1.0, self.log_scale, Message::LogScaleChanged)
                .step(0.01)
                .width(100),
            text(format!("{:.2}", self.log_scale)).size(13).width(35),
        ]
        .spacing(6)
        .align_y(Center);

        // 窗函数选择
        let window_options = [
            FFTWindow::Rectangular,
            FFTWindow::Hanning,
            FFTWindow::Hamming,
            FFTWindow::Blackman,
        ];
        let window_control = row![
            text("窗函数").size(13).style(|_| text::Style { color: Some(Color::from_rgb(0.6, 0.6, 0.7)) }),
            pick_list(window_options, Some(self.window_func), Message::WindowFuncChanged)
                .text_size(13)
                .padding([3, 6]),
        ]
        .spacing(6)
        .align_y(Center);

        // Y轴范围信息
        let scale_info = {
            let range_pct = (self.scale_b - self.scale_a) * 100.0;
            text(format!(
                "Y: {:.0}%-{:.0}% ({:.0}%)", 
                self.scale_a * 100.0, 
                self.scale_b * 100.0,
                range_pct
            ))
            .size(12)
            .style(|_| text::Style { color: Some(Color::from_rgb(0.5, 0.5, 0.6)) })
        };

        // 状态信息
        let status_text = if let Some(ref err) = self.error_msg {
            text(err).size(12).style(text::danger)
        } else if self.scene.is_paused() {
            text("⏸ 暂停中").size(12).style(|_| text::Style { color: Some(Color::from_rgb(0.9, 0.7, 0.2)) })
        } else if self.audio_running {
            text("● 录音中").size(12).style(|_| text::Style { color: Some(Color::from_rgb(0.3, 0.8, 0.3)) })
        } else {
            text("○ 已停止").size(12).style(|_| text::Style { color: Some(Color::from_rgb(0.5, 0.5, 0.6)) })
        };

        // 快捷键提示
        let shortcuts = text("拖拽平移 | 滚轮缩放")
            .size(11)
            .style(|_| text::Style { color: Some(Color::from_rgb(0.4, 0.4, 0.5)) });

        // 单行紧凑布局
        row![
            toggle_btn,
            pause_btn,
            reset_btn,
            horizontal_space().width(15),
            fft_size_control,
            gain_control,
            log_scale_control,
            window_control,
            horizontal_space(),
            column![
                row![scale_info, horizontal_space().width(10), status_text].align_y(Center),
                shortcuts,
            ]
            .spacing(2)
            .align_x(iced::alignment::Horizontal::Right),
        ]
        .spacing(8)
        .align_y(Center)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            // 60fps 定时刷新
            time::every(Duration::from_millis(16)).map(Message::Tick),
            // 键盘事件
            event::listen_with(|event, _status, _id| {
                if let Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event {
                    Some(Message::KeyPressed(key))
                } else {
                    None
                }
            }),
        ])
    }
}
