//! LSP progress presentation for the active preview tab.
use iced::{Element, Theme};
fn is_dark_theme(theme: &Theme) -> bool {
    let palette = theme.palette();
    palette.background.r + palette.background.g + palette.background.b < 1.5
}

#[cfg(not(target_arch = "wasm32"))]
pub fn lsp_content<'a, M: 'a>(
    active_preview_path: Option<&str>, preview_tabs: &[crate::preview::PreviewTab],
    lsp_progress: &std::collections::HashMap<String, std::collections::HashMap<String, crate::preview::LspProgress>>,
    spinner_frame: usize, desktop_locale: config_ui::DesktopLocale, status: Option<String>,
) -> Element<'a, M> {
    // 获取当前激活的预览标签页
    let active_tab = active_preview_path
        .and_then(|path| preview_tabs.iter().find(|t| t.path == path));

    // 构建 LSP 状态显示内容
    let lsp_content: Element<'_, M> = if let Some(tab) = active_tab {
        if let Some(key) = tab.lsp_server_key {
            if let Some(progress_map) = lsp_progress.get(key) {
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
                            iced::widget::text(desktop_i18n::other::lsp_finishing(
                                desktop_i18n::I18n::new(desktop_locale),
                                key
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
                    } else {
                        let percent =
                            progress.percentage.map(|p| format!(" {}%", p)).unwrap_or_default();
                        let msg = progress
                            .message
                            .as_ref()
                            .map(|m| format!(": {}", m))
                            .unwrap_or_default();
                        let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
                        let frame = frames[spinner_frame % frames.len()];
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
            } else if let Some(status) = status.clone() {
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
        } else if let Some(status) = status.clone() {
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
            iced::widget::text(desktop_i18n::I18n::new(desktop_locale).__("就绪"))
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
        iced::widget::text(desktop_i18n::I18n::new(desktop_locale).__("就绪"))
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

    lsp_content
}
