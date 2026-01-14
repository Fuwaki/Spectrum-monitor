#![windows_subsystem = "windows"]

mod amplitude_axis;
mod audio;
mod axis;
mod pipeline;
mod spectrum;
mod spectrum_line;
mod spectrum_overlay;

use amplitude_axis::AmplitudeAxis;
use audio::{Audio, FFTWindow};
use axis::Axis;
use spectrum::SpectrumScene;
use spectrum_line::SpectrumLineScene;
use spectrum_overlay::SpectrumOverlay;

use iced::event::{self, Event};
use iced::keyboard;
use iced::mouse;
use iced::time::{self, Duration, Instant};
use iced::widget::Canvas;
use iced::widget::{
    button, column, container, horizontal_space, mouse_area, pick_list, row, slider, stack, text,
    vertical_space,
};
use iced::window;
use iced::{Center, Color, Element, Fill, Font, Subscription, Task, Theme};

// 嵌入中文字体
const APP_FONT_BYTES: &[u8] = include_bytes!("../Alibaba-PuHuiTi-Heavy.ttf");

// 定义字体常量
// 注：该字体文件的 family 为 "Alibaba PuHuiTi"（PostScript: "AlibabaPuHuiTi-Heavy"）
const APP_FONT: Font = Font {
    family: iced::font::Family::Name("Alibaba PuHuiTi"),
    weight: iced::font::Weight::Black,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

mod style {
    use iced::{
        widget::{button, container},
        Color, Theme,
    };

    pub const BG_DARK: Color = Color::from_rgb(0.06, 0.07, 0.09);
    pub const BG_PANEL: Color = Color::from_rgb(0.09, 0.10, 0.12);
    // Flat/Modern Colors
    pub const ACCENT: Color = Color::from_rgb(0.38, 0.50, 1.0); // Vivid Blue
    pub const ACCENT_DIM: Color = Color::from_rgb(0.25, 0.35, 0.70);
    pub const SUCCESS: Color = Color::from_rgb(0.20, 0.70, 0.45);
    pub const DANGER: Color = Color::from_rgb(0.85, 0.30, 0.35);

    pub const TEXT: Color = Color::from_rgb(0.92, 0.93, 0.95);
    pub const TEXT_MUTED: Color = Color::from_rgb(0.55, 0.58, 0.65);
    pub const BORDER: Color = Color::from_rgb(0.18, 0.20, 0.24);

    pub fn panel(_theme: &Theme) -> container::Style {
        container::Style {
            background: Some(BG_PANEL.into()),
            border: iced::Border {
                color: BORDER,
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub fn chart(_theme: &Theme) -> container::Style {
        container::Style {
            background: Some(BG_DARK.into()),
            border: iced::Border {
                color: BORDER,
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub fn btn_primary(_theme: &Theme, status: button::Status) -> button::Style {
        let base = button::Style {
            background: Some(ACCENT_DIM.into()),
            text_color: TEXT,
            border: iced::Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        };
        match status {
            button::Status::Disabled => button::Style {
                background: Some(Color::from_rgba(0.25, 0.35, 0.70, 0.35).into()),
                text_color: Color::from_rgba(TEXT_MUTED.r, TEXT_MUTED.g, TEXT_MUTED.b, 0.7),
                ..base
            },
            button::Status::Hovered => button::Style {
                background: Some(ACCENT.into()),
                text_color: Color::WHITE,
                ..base
            },
            button::Status::Pressed => button::Style {
                background: Some(ACCENT.into()),
                ..base
            },
            _ => base,
        }
    }

    pub fn btn_success(_theme: &Theme, status: button::Status) -> button::Style {
        let base = button::Style {
            background: Some(Color::from_rgb(0.15, 0.5, 0.3).into()),
            text_color: TEXT,
            border: iced::Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        };
        match status {
            button::Status::Disabled => button::Style {
                background: Some(Color::from_rgba(0.15, 0.5, 0.3, 0.35).into()),
                text_color: Color::from_rgba(TEXT_MUTED.r, TEXT_MUTED.g, TEXT_MUTED.b, 0.7),
                ..base
            },
            button::Status::Hovered => button::Style {
                background: Some(SUCCESS.into()),
                text_color: Color::WHITE,
                ..base
            },
            _ => base,
        }
    }

    pub fn btn_danger(_theme: &Theme, status: button::Status) -> button::Style {
        let base = button::Style {
            background: Some(Color::from_rgb(0.6, 0.2, 0.25).into()),
            text_color: TEXT,
            border: iced::Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        };
        match status {
            button::Status::Disabled => button::Style {
                background: Some(Color::from_rgba(0.6, 0.2, 0.25, 0.35).into()),
                text_color: Color::from_rgba(TEXT_MUTED.r, TEXT_MUTED.g, TEXT_MUTED.b, 0.7),
                ..base
            },
            button::Status::Hovered => button::Style {
                background: Some(DANGER.into()),
                text_color: Color::WHITE,
                ..base
            },
            _ => base,
        }
    }

    pub fn btn_ghost(_theme: &Theme, status: button::Status) -> button::Style {
        let base = button::Style {
            background: None,
            text_color: TEXT_MUTED,
            border: iced::Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        };
        match status {
            button::Status::Disabled => button::Style {
                background: None,
                text_color: Color::from_rgba(TEXT_MUTED.r, TEXT_MUTED.g, TEXT_MUTED.b, 0.55),
                ..base
            },
            button::Status::Hovered => button::Style {
                background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.08).into()),
                text_color: TEXT,
                ..base
            },
            button::Status::Pressed => button::Style {
                background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.12).into()),
                text_color: TEXT,
                ..base
            },
            _ => base,
        }
    }
}

fn main() -> iced::Result {
    env_logger::init();

    iced::application("频谱监视器", App::update, App::view)
        .subscription(App::subscription)
        .theme(|_| Theme::TokyoNightStorm)
        .font(APP_FONT_BYTES)
        .default_font(APP_FONT)
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
    // 窗口大小变化
    WindowResized(iced::Size),
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
    // 窗口尺寸（用于计算视口高度）
    window_size: iced::Size,
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
                window_size: iced::Size::new(1200.0, 700.0), // 初始窗口大小
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

                    // 停止时取消暂停，避免状态冲突
                    if self.scene.is_paused() {
                        self.scene.toggle_pause();
                    }
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
                if self.audio_running {
                    self.scene.toggle_pause();
                }
            }
            Message::KeyPressed(key) => {
                use iced::keyboard::key;

                match key {
                    keyboard::Key::Named(key::Named::Space) => {
                        if self.audio_running {
                            self.scene.toggle_pause();
                        }
                    }
                    keyboard::Key::Character(ch) => {
                        let lower = ch.to_lowercase();
                        match lower.as_str() {
                            "s" => {
                                // Start/Stop
                                if self.audio_running {
                                    self.audio.stop();
                                    self.audio_running = false;
                                    if self.scene.is_paused() {
                                        self.scene.toggle_pause();
                                    }
                                } else if let Err(e) = self.audio.start() {
                                    self.error_msg = Some(format!("音频启动失败: {}", e));
                                } else {
                                    self.audio_running = true;
                                    self.error_msg = None;
                                }
                            }
                            "r" => {
                                // Reset view (still useful even without a button)
                                self.scale_a = 0.0;
                                self.scale_b = 1.0;
                                self.scene.set_scale(self.scale_a, self.scale_b);
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            Message::ResetScale => {
                self.scale_a = 0.0;
                self.scale_b = 1.0;
                self.scene.set_scale(self.scale_a, self.scale_b);
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
                // 计算频谱区域的实际高度 (窗口高度 - 控制面板 - 边距)
                let spectrum_height = self.window_size.height - 120.0; // 控制面板约100px + 边距

                if self.is_dragging {
                    if let Some(last_pos) = self.last_mouse_pos {
                        let dy = position.y - last_pos.y;

                        // 拖拽平移 - 只移动视图，不改变缩放范围
                        // 移动距离相对于视口高度的比例，映射到当前scale范围
                        let range = self.scale_b - self.scale_a;
                        let move_delta = (dy / spectrum_height) * range;

                        // 鼠标向下拖 (dy > 0) -> 内容向上滚动 -> scale增加
                        let new_a = self.scale_a + move_delta;
                        let new_b = self.scale_b + move_delta;

                        // 边界检查，保持范围不变
                        if new_a >= 0.0 && new_b <= 1.0 {
                            self.scale_a = new_a;
                            self.scale_b = new_b;
                        } else if new_a < 0.0 {
                            self.scale_a = 0.0;
                            self.scale_b = range;
                        } else if new_b > 1.0 {
                            self.scale_b = 1.0;
                            self.scale_a = 1.0 - range;
                        }

                        self.scene.set_scale(self.scale_a, self.scale_b);
                    }
                    self.last_mouse_pos = Some(position);
                } else {
                    // 记录鼠标位置，用于滚轮缩放时的中心计算
                    self.last_mouse_pos = Some(position);
                }
            }
            Message::MouseInteraction(mouse::Event::WheelScrolled { delta }) => {
                // 鼠标滚轮缩放Y轴 - 以鼠标位置为中心
                const TOUCHPAD_SENSITIVITY: f32 = 0.002;
                const MOUSEWHEEL_SENSITIVITY: f32 = 0.1;

                let d = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y * MOUSEWHEEL_SENSITIVITY,
                    mouse::ScrollDelta::Pixels { y, .. } => y * TOUCHPAD_SENSITIVITY,
                };

                if d == 0.0 {
                    return Task::none();
                }

                // 计算频谱区域的实际高度 (窗口高度 - 控制面板 - 边距)
                let spectrum_height = self.window_size.height - 120.0;

                let range = self.scale_b - self.scale_a;

                // 计算鼠标在视口中的相对位置 (0=顶部, 1=底部)
                let relative_y = if let Some(pos) = self.last_mouse_pos {
                    (pos.y / spectrum_height).clamp(0.0, 1.0)
                } else {
                    0.5 // 默认中心
                };

                // 将相对Y位置转换为scale空间中的位置
                // 屏幕顶部(y=0) -> scale_b (高频), 屏幕底部(y=height) -> scale_a (低频)
                let zoom_center = self.scale_b - relative_y * range;

                // 计算新的范围 (d > 0 放大，范围减小)
                let new_range = (range * (1.0 - d * 0.5)).clamp(0.001, 1.0);

                // 以鼠标位置为中心计算新的边界
                // 保持鼠标指向的频率在新视图中的相对位置不变
                let new_a = zoom_center - (1.0 - relative_y) * new_range;
                let new_b = zoom_center + relative_y * new_range;

                // 边界调整 - 优先保持鼠标位置不变
                let (final_a, final_b) = if new_a < 0.0 {
                    // 碰到下边界，调整但尝试保持缩放中心
                    (0.0, new_range.min(1.0))
                } else if new_b > 1.0 {
                    // 碰到上边界
                    ((1.0 - new_range).max(0.0), 1.0)
                } else {
                    (new_a, new_b)
                };

                self.scale_a = final_a;
                self.scale_b = final_b;

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
                    // 使用实际窗口宽度计算
                    let axis_width = 50.0;
                    let padding = 20.0;
                    let available_width = self.window_size.width - axis_width - padding * 2.0;
                    let relative_x = position.x - axis_width - padding;

                    if available_width > 0.0 && relative_x > 0.0 {
                        let ratio = (relative_x / available_width).clamp(0.2, 0.9);
                        self.spectrum_ratio = ratio;
                    }
                }
            }
            Message::WindowResized(size) => {
                self.window_size = size;
            }
            _ => {}
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        // 1. 折线图 (Line Plot)
        let line_scene = SpectrumLineScene { scene: &self.scene };
        let line_plot = iced::widget::shader(line_scene).width(Fill).height(Fill);

        // Overlay & Axis
        let (
            overlay_data,
            overlay_data_length,
            overlay_gain,
            overlay_log_scale,
            overlay_scale_a,
            overlay_scale_b,
        ) = self.scene.get_line_data();
        let overlay = Canvas::new(SpectrumOverlay {
            data: overlay_data,
            data_length: overlay_data_length,
            scale_a: overlay_scale_a,
            scale_b: overlay_scale_b,
            log_scale: overlay_log_scale,
            gain: overlay_gain,
            max_freq: 22050.0,
        })
        .width(Fill)
        .height(Fill);

        let line_plot_stack = stack![
            line_plot,
            overlay,
            column![vertical_space(), AmplitudeAxis::new().view()]
        ];

        // 2. 瀑布图 (Spectrogram)
        let spectrum_shader = iced::widget::shader(&self.scene).width(Fill).height(Fill);

        // 可拖拽的分隔条
        let splitter = mouse_area(
            container(horizontal_space())
                .width(4) // Thinner splitter
                .height(Fill)
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(if self.is_split_dragging {
                        style::ACCENT
                    } else {
                        Color::from_rgba(0.2, 0.2, 0.25, 0.5)
                    })),
                    ..Default::default()
                }),
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
                self.axis
                    .view(20.0, 22050.0, self.scale_a, self.scale_b, self.log_scale),
                // Spectrogram - 左边
                container(spectrum_shader)
                    .width(spectrum_width)
                    .style(style::chart),
                // Spliter
                splitter,
                // Line Plot - 右边
                container(line_plot_stack)
                    .width(line_width)
                    .style(style::chart),
            ]
            .spacing(0),
        )
        .on_press(Message::MouseInteraction(mouse::Event::ButtonPressed(
            mouse::Button::Left,
        )))
        .on_release(Message::MouseInteraction(mouse::Event::ButtonReleased(
            mouse::Button::Left,
        )))
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
                    text("已暂停")
                        .size(24)
                        .font(APP_FONT)
                        .style(|_| text::Style {
                            color: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.5)),
                        }),
                )
                .padding([10, 20])
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgba(
                        0.0, 0.0, 0.0, 0.4,
                    ))),
                    border: iced::Border {
                        radius: 8.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            )
        } else {
            None
        };

        let spectrum_content = if let Some(overlay) = pause_overlay {
            container(stack![
                spectrum_area,
                container(overlay)
                    .width(Fill)
                    .height(Fill)
                    .align_x(Center)
                    .align_y(Center)
            ])
            .width(Fill)
            .height(Fill)
        } else {
            container(spectrum_area).width(Fill).height(Fill)
        };

        // 控制面板
        let controls = self.view_controls();

        // 主布局 - Remove outer padding for flat look
        column![
            spectrum_content,
            container(controls).style(style::panel), // Bottom Panel Style
        ]
        .spacing(0)
        .into()
    }

    fn view_controls(&self) -> Element<'_, Message> {
        // 1. Playback Group
        let toggle_btn = button(
            text(if self.audio_running {
                " 停止 "
            } else {
                " 开始 "
            })
            .size(12)
            .font(APP_FONT),
        )
        .padding([8, 16])
        .on_press(Message::ToggleAudio)
        .style(if self.audio_running {
            style::btn_danger
        } else {
            style::btn_success
        });

        let mut pause_button = button(
            text(if self.scene.is_paused() {
                " 继续 "
            } else {
                " 暂停 "
            })
            .size(12),
        )
        .padding([8, 16])
        .style(if self.scene.is_paused() {
            style::btn_primary
        } else {
            style::btn_ghost
        });

        if self.audio_running {
            pause_button = pause_button.on_press(Message::TogglePause);
        }

        let pause_btn = pause_button;

        // 2. Settings Group
        let separator = || {
            container(horizontal_space())
                .width(1)
                .height(24)
                .style(|_| container::Style {
                    background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.1).into()),
                    ..Default::default()
                })
        };

        // Helper for sliders
        macro_rules! slider_group {
            ($label:expr, $value:expr, $control:expr) => {
                column![
                    row![
                        text($label).size(10).style(|_| text::Style {
                            color: Some(style::TEXT_MUTED)
                        }),
                        horizontal_space(),
                        text($value).size(10).style(|_| text::Style {
                            color: Some(style::ACCENT)
                        }),
                    ]
                    .width(Fill),
                    $control
                ]
                .spacing(2)
                .width(130)
            };
        }

        let fft_control = slider_group!(
            "FFT 大小",
            format!("{}", self.fft_size),
            slider(8.0..=14.0, (self.fft_size as f32).log2(), |v| {
                Message::FFTSizeChanged(2u32.pow(v as u32))
            })
            .step(1.0)
            .height(20)
        );

        let gain_control = slider_group!(
            "增益",
            format!("{:.2}", self.gain),
            slider(0.01..=1.0, self.gain, Message::GainChanged)
                .step(0.01)
                .height(20)
        );

        let log_control = slider_group!(
            "对数刻度",
            format!("{:.2}", self.log_scale),
            slider(0.0..=1.0, self.log_scale, Message::LogScaleChanged)
                .step(0.01)
                .height(20)
        );

        // Window function
        let window_options = [
            FFTWindow::Rectangular,
            FFTWindow::Hanning,
            FFTWindow::Hamming,
            FFTWindow::Blackman,
        ];
        let window_control = column![
            text("窗函数").size(10).style(|_| text::Style {
                color: Some(style::TEXT_MUTED)
            }),
            pick_list(
                window_options,
                Some(self.window_func),
                Message::WindowFuncChanged
            )
            .text_size(12)
            .padding([4, 8])
            .width(Fill)
        ]
        .spacing(2)
        .width(110);

        // 3. Status Group
        let status_text = if let Some(ref err) = self.error_msg {
            row![
                text("⚠").style(text::danger).size(12),
                text(err).size(12).style(text::danger)
            ]
            .spacing(4)
        } else if self.scene.is_paused() {
            row![
                text("⏸").size(12).style(|_| text::Style {
                    color: Some(style::TEXT_MUTED)
                }),
                text("已暂停").size(12).style(|_| text::Style {
                    color: Some(style::TEXT_MUTED)
                })
            ]
            .spacing(4)
        } else if self.audio_running {
            row![
                text("●").size(12).style(|_| text::Style {
                    color: Some(style::SUCCESS)
                }),
                text("实时").size(12).style(|_| text::Style {
                    color: Some(style::SUCCESS)
                })
            ]
            .spacing(4)
        } else {
            row![
                text("○").size(12).style(|_| text::Style {
                    color: Some(style::TEXT_MUTED)
                }),
                text("就绪").size(12).style(|_| text::Style {
                    color: Some(style::TEXT_MUTED)
                })
            ]
            .spacing(4)
        };

        let scale_info = {
            let range_pct = (self.scale_b - self.scale_a) * 100.0;
            text(format!("缩放: {:.0}%", range_pct))
                .size(11)
                .style(|_| text::Style {
                    color: Some(style::TEXT_MUTED),
                })
        };

        row![
            // Left Controls
            row![toggle_btn, pause_btn].spacing(8).align_y(Center),
            separator(),
            // Params
            row![fft_control, gain_control, log_control, window_control]
                .spacing(16)
                .align_y(Center),
            horizontal_space(),
            // Status
            column![status_text, scale_info]
                .spacing(2)
                .align_x(iced::alignment::Horizontal::Right)
        ]
        .spacing(16)
        .padding([12, 24])
        .align_y(Center)
        .height(70)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            // 60fps 定时刷新
            time::every(Duration::from_millis(16)).map(Message::Tick),
            // 键盘和窗口事件
            event::listen_with(|event, _status, _id| match event {
                Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                    Some(Message::KeyPressed(key))
                }
                Event::Window(window::Event::Resized(size)) => Some(Message::WindowResized(size)),
                _ => None,
            }),
        ])
    }
}
