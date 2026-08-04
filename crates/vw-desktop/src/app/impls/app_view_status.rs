//! 应用状态栏视图模块
//!
//! 本模块提供应用程序底部状态栏的渲染功能，主要展示以下信息：
//! - LSP（语言服务器协议）的状态和进度
//! - 通知图标和计数器
//!
//! 状态栏会根据当前激活的预览标签页动态显示相关的 LSP 信息，
//! 包括语言服务器的工作进度、完成状态以及通用状态信息。

use iced::Element;
#[cfg(not(target_arch = "wasm32"))]
use iced::Theme;
#[cfg(not(target_arch = "wasm32"))]
use iced::widget::button;
#[cfg(not(target_arch = "wasm32"))]
use iced::{Background, Color};

use super::{App, Message};

pub(crate) const STATUS_BAR_HEIGHT: f32 = 30.0;

#[cfg(not(target_arch = "wasm32"))]
fn is_dark_theme(theme: &Theme) -> bool {
    let palette = theme.palette();
    palette.background.r + palette.background.g + palette.background.b < 1.5
}

/// 构建应用程序状态栏
///
/// 该函数为非 WebAssembly 平台创建状态栏 UI 组件。状态栏包含以下元素：
///
/// # 布局结构
/// - 左侧：账户、LSP 状态信息（包括进度指示器和状态文本）
/// - 中间：弹性空白填充
/// - 右侧：通知按钮（显示图标和未读计数）
///
/// # LSP 状态显示逻辑
/// 1. 优先显示当前激活标签页的 LSP 进度（如果有）
/// 2. 显示通用 LSP 状态（如果存在）
/// 3. 默认显示"就绪"状态
///
/// # 参数
/// - `app`: 应用程序状态引用，包含所有需要显示的信息
///
/// # 返回值
/// 返回一个 Iced Element，代表渲染的状态栏组件
///
/// # 平台兼容性
/// 此函数仅在非 wasm32 目标平台上可用
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn status_bar(app: &App) -> Element<'_, Message> {
    // 获取当前激活的预览标签页
    let active_tab = app
        .active_preview_path
        .as_deref()
        .and_then(|path| app.preview_tabs.iter().find(|t| t.path == path));

    // 构建 LSP 状态显示内容
    let lsp_content: Element<'_, Message> = if let Some(tab) = active_tab {
        if let Some(key) = tab.lsp_server_key {
            if let Some(progress_map) = app.lsp_progress.get(key) {
                if let Some(progress) = progress_map.values().next() {
                    let percent_val = progress.percentage.unwrap_or(0);
                    if percent_val >= 100 {
                        iced::widget::row![
                            iced::widget::text("✓").size(14).style(move |theme: &iced::Theme| {
                                let palette = theme.extended_palette();
                                iced::widget::text::Style {
                                    color: Some(palette.success.base.color),
                                }
                            }),
                            iced::widget::text(format!("LSP: {} (即将完成...)", key))
                                .size(12)
                                .style(move |theme: &iced::Theme| {
                                    let palette = theme.extended_palette();
                                    iced::widget::text::Style {
                                        color: Some(if is_dark_theme(theme) {
                                            palette.background.base.text.scale_alpha(0.84)
                                        } else {
                                            theme.palette().text.scale_alpha(0.74)
                                        }),
                                    }
                                })
                        ]
                        .spacing(5)
                        .align_y(iced::Alignment::Center)
                        .into()
                    } else {
                        let percent =
                            progress.percentage.map(|p| format!(" {}%", p)).unwrap_or_default();
                        let msg = progress
                            .message
                            .as_ref()
                            .map(|m| format!(": {}", m))
                            .unwrap_or_default();
                        let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
                        let frame = frames[app.spinner_frame % frames.len()];
                        iced::widget::row![
                            iced::widget::text(frame)
                                .size(14)
                                .font(iced::font::Font::MONOSPACE)
                                .style(move |theme: &iced::Theme| {
                                    let palette = theme.extended_palette();
                                    iced::widget::text::Style {
                                        color: Some(palette.primary.base.color),
                                    }
                                }),
                            iced::widget::text(format!(
                                "LSP: {} ({}{}{})",
                                key, progress.title, msg, percent
                            ))
                            .size(12)
                            .style(move |theme: &iced::Theme| {
                                let palette = theme.extended_palette();
                                iced::widget::text::Style {
                                    color: Some(if is_dark_theme(theme) {
                                        palette.background.base.text.scale_alpha(0.84)
                                    } else {
                                        theme.palette().text.scale_alpha(0.74)
                                    }),
                                }
                            })
                        ]
                        .spacing(5)
                        .align_y(iced::Alignment::Center)
                        .into()
                    }
                } else {
                    iced::widget::text(format!("LSP: {}", key))
                        .size(12)
                        .style(move |theme: &iced::Theme| {
                            let palette = theme.extended_palette();
                            iced::widget::text::Style {
                                color: Some(if is_dark_theme(theme) {
                                    palette.background.base.text.scale_alpha(0.82)
                                } else {
                                    theme.palette().text.scale_alpha(0.72)
                                }),
                            }
                        })
                        .into()
                }
            } else if let Some(status) = &app.lsp_status {
                iced::widget::text(status)
                    .size(12)
                    .style(move |theme: &iced::Theme| {
                        let palette = theme.extended_palette();
                        iced::widget::text::Style {
                            color: Some(if is_dark_theme(theme) {
                                palette.background.base.text.scale_alpha(0.82)
                            } else {
                                theme.palette().text.scale_alpha(0.72)
                            }),
                        }
                    })
                    .into()
            } else {
                iced::widget::text(format!("LSP: {}", key))
                    .size(12)
                    .style(move |theme: &iced::Theme| {
                        let palette = theme.extended_palette();
                        iced::widget::text::Style {
                            color: Some(if is_dark_theme(theme) {
                                palette.background.base.text.scale_alpha(0.82)
                            } else {
                                theme.palette().text.scale_alpha(0.72)
                            }),
                        }
                    })
                    .into()
            }
        } else if let Some(status) = &app.lsp_status {
            iced::widget::text(status)
                .size(12)
                .style(move |theme: &iced::Theme| {
                    let palette = theme.extended_palette();
                    iced::widget::text::Style {
                        color: Some(if is_dark_theme(theme) {
                            palette.background.base.text.scale_alpha(0.82)
                        } else {
                            theme.palette().text.scale_alpha(0.72)
                        }),
                    }
                })
                .into()
        } else {
            iced::widget::text("就绪")
                .size(12)
                .style(move |theme: &iced::Theme| {
                    let palette = theme.extended_palette();
                    iced::widget::text::Style {
                        color: Some(if is_dark_theme(theme) {
                            palette.background.base.text.scale_alpha(0.82)
                        } else {
                            theme.palette().text.scale_alpha(0.72)
                        }),
                    }
                })
                .into()
        }
    } else {
        iced::widget::text("就绪")
            .size(12)
            .style(move |theme: &iced::Theme| {
                let palette = theme.extended_palette();
                iced::widget::text::Style {
                    color: Some(if is_dark_theme(theme) {
                        palette.background.base.text.scale_alpha(0.82)
                    } else {
                        theme.palette().text.scale_alpha(0.72)
                    }),
                }
            })
            .into()
    };

    // 构建通知按钮区域
    let notification_btn = {
        let count = app.notifications.len();

        let icon: Element<'_, Message> = if count > 0 {
            iced::widget::text("⚠️")
                .size(14)
                .style(move |theme: &iced::Theme| {
                    let p = theme.extended_palette();
                    iced::widget::text::Style { color: Some(p.warning.base.color) }
                })
                .into()
        } else {
            iced::widget::text("🔔")
                .size(14)
                .style(move |theme: &iced::Theme| {
                    let p = theme.extended_palette();
                    iced::widget::text::Style {
                        color: Some(if is_dark_theme(theme) {
                            p.background.base.text.scale_alpha(0.72)
                        } else {
                            theme.palette().text.scale_alpha(0.64)
                        }),
                    }
                })
                .into()
        };

        let count_el: Element<'_, Message> = if count > 0 {
            iced::widget::text(count.to_string())
                .size(12)
                .style(move |theme: &iced::Theme| {
                    let p = theme.extended_palette();
                    iced::widget::text::Style { color: Some(p.warning.base.color) }
                })
                .into()
        } else {
            iced::widget::Space::new()
                .width(iced::Length::Fixed(0.0))
                .height(iced::Length::Fixed(0.0))
                .into()
        };

        button(iced::widget::row![icon, count_el].spacing(4).align_y(iced::Alignment::Center))
            .on_press(Message::Notification(
                crate::app::message::NotificationMessage::ToggleExpanded,
            ))
            .style(|theme: &iced::Theme, status| {
                let p = theme.extended_palette();
                iced::widget::button::Style {
                    background: match status {
                        iced::widget::button::Status::Hovered => {
                            Some(Background::Color(if is_dark_theme(theme) {
                                p.background.weak.color.scale_alpha(0.42)
                            } else {
                                Color::WHITE.scale_alpha(0.52)
                            }))
                        }
                        iced::widget::button::Status::Pressed => {
                            Some(Background::Color(if is_dark_theme(theme) {
                                p.background.strong.color.scale_alpha(0.54)
                            } else {
                                p.background.weak.color.scale_alpha(0.76)
                            }))
                        }
                        _ => None,
                    },
                    text_color: if is_dark_theme(theme) {
                        p.background.base.text.scale_alpha(0.80)
                    } else {
                        theme.palette().text.scale_alpha(0.72)
                    },
                    ..Default::default()
                }
            })
    };
    let account = crate::app::components::account_status::view(app);

    // 组装完整的状态栏布局
    let content = iced::widget::row![
        account,
        lsp_content,
        iced::widget::Space::new().width(iced::Length::Fill),
        notification_btn
    ]
    .align_y(iced::Alignment::Center)
    .spacing(10);

    iced::widget::container(content)
        .width(iced::Length::Fill)
        .height(iced::Length::Fixed(STATUS_BAR_HEIGHT))
        .padding([2, 8])
        .style(|theme: &iced::Theme| {
            let p = theme.extended_palette();
            iced::widget::container::Style {
                background: Some(Background::Color(if is_dark_theme(theme) {
                    p.background.weak.color.scale_alpha(0.78)
                } else {
                    Color::from_rgba8(248, 250, 253, 0.96)
                })),
                text_color: Some(if is_dark_theme(theme) {
                    p.background.base.text.scale_alpha(0.82)
                } else {
                    theme.palette().text.scale_alpha(0.72)
                }),
                ..Default::default()
            }
        })
        .into()
}

/// 构建 WebAssembly 平台的状态栏
///
/// 该函数为 WebAssembly 目标平台提供状态栏实现。
/// 在 WebAssembly 环境中，状态栏被简化为零高度的占位符，
/// 以减少资源消耗并简化 Web 环境下的渲染逻辑。
///
/// # 参数
/// - `_app`: 应用程序状态引用（在 WebAssembly 版本中未使用）
///
/// # 返回值
/// 返回一个零高度的空白 Element
///
/// # 平台兼容性
/// 此函数仅在 wasm32 目标平台上可用
#[cfg(target_arch = "wasm32")]
pub(super) fn status_bar(_app: &App) -> Element<'_, Message> {
    iced::widget::Space::new().height(0).into()
}
#[cfg(test)]
#[path = "app_view_status_tests.rs"]
mod app_view_status_tests;
