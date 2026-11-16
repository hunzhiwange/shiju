//! 补丁卡片控件、路径摘要与删除文件预览。
use std::path::Path;
use iced::widget::{button, container, row, svg, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};
use desktop_assets::assets::{self, Icon};
use crate::card_input::{CardInput, Message};
use crate::card_style::{bold_font, icon_svg};
use super::types::ChangeFile;
use super::apply_patch_preview::find_apply_patch_change;
#[cfg(not(target_arch = "wasm32"))]
use super::apply_patch_preview::parse_unified_diff_change_files;
fn view_changes_pill<'a, Host: Clone + 'static>(i18n: desktop_i18n::I18n) -> Element<'a, Message<Host>> {
    container(text(i18n.__("查看变更")).size(14).style(|theme: &Theme| {
        let is_dark = theme.palette().background.r
            + theme.palette().background.g
            + theme.palette().background.b
            < 1.5;
        iced::widget::text::Style {
            color: Some(if is_dark {
                theme.palette().text.scale_alpha(0.88)
            } else {
                theme.extended_palette().secondary.base.text
            }),
        }
    }))
    .padding([2, 8])
    .style(|theme: &Theme| {
        let ext = theme.extended_palette();
        let is_dark = theme.palette().background.r
            + theme.palette().background.g
            + theme.palette().background.b
            < 1.5;
        iced::widget::container::Style {
            background: Some(Background::Color(if is_dark {
                ext.background.weak.color.scale_alpha(0.24)
            } else {
                ext.background.base.color.scale_alpha(0.76)
            })),
            border: Border {
                width: 1.0,
                color: ext.background.strong.color.scale_alpha(if is_dark { 0.48 } else { 0.75 }),
                radius: 999.0.into(),
            },
            ..Default::default()
        }
    })
    .into()
}

pub(super) fn view_changes_toggle_button<'a, Host: Clone + 'static>(expanded: bool, on_press: Message<Host>) -> Element<'a, Message<Host>> {
    button(
        icon_svg(if expanded { Icon::ChevronDown } else { Icon::ChevronRight })
            .width(Length::Fixed(10.0))
            .height(Length::Fixed(10.0))
            .style(|theme: &Theme, _status| svg::Style {
                color: Some(apply_patch_secondary_text(theme, 0.72, 0.74)),
            }),
    )
    .padding([2, 4])
    .style(apply_patch_file_button_style)
    .on_press(on_press)
    .into()
}

pub(super) fn external_diff_button<'a, Host: Clone + 'static>(
    i18n: desktop_i18n::I18n,
    on_press: Message<Host>,
) -> Element<'a, Message<Host>> {
    button(view_changes_pill(i18n))
        .padding(0)
        .style(apply_patch_file_button_style)
        .on_press(on_press)
        .into()
}

pub(super) fn resolve_apply_patch_change_preview(
    app: &CardInput<'_>,
    changes: &[ChangeFile],
    kind: char,
    path: &str,
) -> Option<ChangeFile> {
    let resolved = find_apply_patch_change(changes, path).cloned();
    #[cfg(not(target_arch = "wasm32"))]
    let resolved = resolved
        .or_else(|| (kind == 'D').then(|| deleted_file_preview_from_repo_diff(app.project_path, path)).flatten())
        .or_else(|| (kind == 'D').then(|| deleted_file_preview_from_history(app.project_path, path)).flatten())
        .or_else(|| {
            let change = find_apply_patch_change(changes, path)?;
            if kind == 'D' && change.before.is_empty() {
                deleted_file_preview_from_repo_diff(app.project_path, path)
                    .or_else(|| deleted_file_preview_from_history(app.project_path, path))
            } else {
                None
            }
        });
    #[cfg(target_arch = "wasm32")]
    let _ = (app, kind);
    resolved
}

pub(crate) fn apply_patch_header_summary(
    i18n: desktop_i18n::I18n,
    files: &[(char, String)],
) -> (String, String, Option<String>) {
    let Some((_, first_path)) = files.first() else {
        return (String::new(), String::new(), None);
    };

    let path = Path::new(first_path);
    let file_name = path
        .file_name()
        .and_then(|segment| segment.to_str())
        .unwrap_or(first_path.as_str())
        .to_string();
    let parent_dir = path
        .parent()
        .and_then(|parent| parent.to_str())
        .filter(|parent| !parent.is_empty() && *parent != ".")
        .map(|parent| format!("/{}", parent.trim_start_matches('/')))
        .unwrap_or_default();
    let extra = if files.len() > 1 {
        Some(
            i18n.format("等 %u 个文件", &[files.len().into()])
                .expect("valid patch summary template"),
        )
    } else {
        None
    };
    (file_name, parent_dir, extra)
}

pub(crate) use super::diff_utils::looks_like_unified_diff as diff_utils_looks_like_unified_diff;

pub(crate) fn apply_patch_secondary_text(
    theme: &Theme,
    dark_alpha: f32,
    light_alpha: f32,
) -> Color {
    let is_dark =
        theme.palette().background.r + theme.palette().background.g + theme.palette().background.b
            < 1.5;
    if is_dark {
        theme.palette().text.scale_alpha(dark_alpha)
    } else {
        theme.extended_palette().secondary.base.text.scale_alpha(light_alpha)
    }
}

pub(crate) fn apply_patch_change_totals<'a, Host: Clone + 'static>(adds: usize, dels: usize) -> Element<'a, Message<Host>> {
    row![
        text(format!("+{}", adds)).size(14).font(bold_font()).style(|theme: &Theme| {
            iced::widget::text::Style { color: Some(theme.extended_palette().success.base.color) }
        }),
        text(format!("-{}", dels)).size(14).font(bold_font()).style(|theme: &Theme| {
            iced::widget::text::Style { color: Some(theme.extended_palette().danger.base.color) }
        })
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

pub(crate) fn apply_patch_outer_card_style(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: None,
        border: Border::default(),
        ..Default::default()
    }
}

pub(crate) fn apply_patch_inner_card_style(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: None,
        border: Border::default(),
        ..Default::default()
    }
}

pub(crate) fn apply_patch_file_button_style(
    theme: &Theme,
    _status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: None,
        border: Border::default(),
        text_color: theme.palette().text,
        ..Default::default()
    }
}

pub(super) fn apply_patch_file_icon<'a, Host: Clone + 'static>(icon: Icon) -> Element<'a, Message<Host>> {
    container(icon_svg(icon).width(Length::Fixed(13.0)).height(Length::Fixed(13.0)).style(
        |_theme: &Theme, _status| svg::Style { color: Some(Color::from_rgb8(0xF4, 0x72, 0x42)) },
    ))
    .padding([4, 5])
    .style(|theme: &Theme| {
        let ext = theme.extended_palette();
        let is_dark = theme.palette().background.r
            + theme.palette().background.g
            + theme.palette().background.b
            < 1.5;
        iced::widget::container::Style {
            background: Some(Background::Color(if is_dark {
                Color::from_rgba(0.96, 0.45, 0.24, 0.16)
            } else {
                Color::from_rgba(0.96, 0.45, 0.24, 0.10)
            })),
            border: Border {
                width: 1.0,
                color: ext.background.strong.color.scale_alpha(if is_dark { 0.36 } else { 0.28 }),
                radius: 8.0.into(),
            },
            ..Default::default()
        }
    })
    .into()
}

pub(super) fn header_file_type_icon(icon: Icon) -> iced::widget::svg::Svg<'static> {
    iced::widget::svg::Svg::new(assets::get_icon(icon))
        .width(Length::Fixed(14.0))
        .height(Length::Fixed(14.0))
}

pub(crate) fn apply_patch_header_file_icon(path: &str) -> Icon {
    let lower = path.to_lowercase();

    if lower.ends_with(".rs") {
        Icon::Rust
    } else if lower.ends_with(".ts") || lower.ends_with(".tsx") {
        Icon::Typescript
    } else if lower.ends_with(".js") || lower.ends_with(".jsx") {
        Icon::Javascript
    } else if lower.ends_with(".json") {
        Icon::Json
    } else if lower.ends_with(".toml") {
        Icon::Toml
    } else if lower.ends_with(".yaml") || lower.ends_with(".yml") {
        Icon::Yaml
    } else if lower.ends_with(".md") {
        Icon::Markdown
    } else if lower.ends_with(".html") || lower.ends_with(".htm") {
        Icon::Html
    } else if lower.ends_with(".css") {
        Icon::Css
    } else if lower.ends_with(".py") {
        Icon::Python
    } else if lower.ends_with(".go") {
        Icon::Go
    } else if lower.ends_with(".sh") {
        Icon::Console
    } else {
        Icon::Document
    }
}

pub(super) fn preview_eye_button<'a, Host: Clone + 'static>(abs: String) -> Element<'a, Message<Host>> {
    let eye = icon_svg(Icon::Eye).width(Length::Fixed(12.0)).height(Length::Fixed(12.0)).style(
        |theme: &Theme, _status| {
            let is_dark = theme.palette().background.r
                + theme.palette().background.g
                + theme.palette().background.b
                < 1.5;
            svg::Style {
                color: Some(if is_dark {
                    theme.palette().text.scale_alpha(0.9)
                } else {
                    theme.extended_palette().secondary.base.text
                }),
            }
        },
    );
    button(eye)
        .padding([2, 4])
        .style(apply_patch_file_button_style)
        .on_press(Message::OpenPreview(abs))
        .into()
}

pub fn apply_patch_file_expand_key(msg_idx: usize, tool_idx: usize, path: &str) -> String {
    format!("{msg_idx}:{tool_idx}:{path}")
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn deleted_file_preview_from_history(root: Option<&str>, path: &str) -> Option<ChangeFile> {
    let root = root?;
    let repo = git2::Repository::open(root).ok()?;
    let head = repo.head().ok()?;
    let tree = head.peel_to_tree().ok()?;
    let entry = tree.get_path(Path::new(path)).ok()?;
    let obj = entry.to_object(&repo).ok()?;
    let blob = obj.as_blob()?;
    let before = String::from_utf8_lossy(blob.content()).to_string();
    let deletions = before.lines().count();
    Some(ChangeFile {
        path: path.to_string(),
        additions: 0,
        deletions,
        before,
        after: String::new(),
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn deleted_file_preview_from_repo_diff(root: Option<&str>, path: &str) -> Option<ChangeFile> {
    let root = root?;
    let repo = git2::Repository::open(root).ok()?;
    let head = repo.head().ok()?;
    let tree = head.peel_to_tree().ok()?;

    let mut opts = git2::DiffOptions::new();
    opts.include_untracked(true);
    opts.recurse_untracked_dirs(true);
    opts.pathspec(path);

    let diff = repo.diff_tree_to_workdir_with_index(Some(&tree), Some(&mut opts)).ok()?;
    let mut patch = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line: git2::DiffLine| {
        patch.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
        true
    })
    .ok()?;

    parse_unified_diff_change_files(&patch)
        .into_iter()
        .find(|change| !change.before.is_empty() && change.after.is_empty() && change.path == path)
}

#[cfg(test)]
#[path = "apply_patch_view_tests.rs"]
mod tests;
