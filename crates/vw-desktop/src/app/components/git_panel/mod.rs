//! Git 面板模块
//!
//! 本模块提供 Git 仓库变更查看和管理的图形界面组件。主要功能包括：
//! - 显示 Git 仓库中的变更文件列表及其 diff 视图
//! - 支持文件过滤（按状态、已暂存/未暂存、搜索关键词）
//! - 自定义文本对比功能
//! - 代码复制和编辑模态框
//! - 行内评论功能
//!
//! 模块结构：
//! - `diff_view`: 变更差异视图渲染
//! - `filter_options`: 过滤选项 UI
//! - `header`: 面板头部
//! - `left_panel`: 左侧面板（汇总视图）
//! - `ops`: Git 操作工具函数
//! - `ui`: UI 组件
//! - `utils`: 工具函数（颜色、文件状态等）

use iced::Element;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    Space, button, column, container, row, scrollable, stack, text, text_editor, text_input,
};
use iced::{Background, Border, Color, Length};
use std::collections::HashSet;

use crate::app::components::system_settings_common::{
    settings_close_button, settings_text_editor_style, settings_text_input_style,
    with_settings_help_modal,
};
use crate::app::{App, DiffTheme, Message, message};

mod diff_view;
mod filter_options;
mod header;
mod left_panel;
mod ops;
mod ui;
mod utils;

#[cfg(test)]
#[path = "filter_options_tests.rs"]
mod filter_options_tests;
#[cfg(test)]
#[path = "header_tests.rs"]
mod header_tests;
#[cfg(test)]
#[path = "left_panel_tests.rs"]
mod left_panel_tests;
#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "ops_tests.rs"]
mod ops_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
#[path = "ui_tests.rs"]
mod ui_tests;
#[cfg(test)]
#[path = "utils_tests.rs"]
mod utils_tests;

pub use ops::{DiffFileMeta, checkout_branch, current_branch, list_branches, open_terminal};

pub(crate) fn diff_comment_modal<'a>(app: &'a App) -> Option<Element<'a, Message>> {
    diff_view::diff_comment_editor(app)
}

pub(crate) fn discard_file_modal<'a>(app: &'a App) -> Option<Element<'a, Message>> {
    let file = app.file_to_discard.as_ref()?.clone();

    let cancel_button = button(text("取消"))
        .on_press(Message::Git(message::GitMessage::CancelDiscardFile))
        .padding([6, 12])
        .style(|theme: &iced::Theme, status| {
            let ext = theme.extended_palette();
            let bg = match status {
                iced::widget::button::Status::Hovered => {
                    ext.background.strong.color.scale_alpha(0.30)
                }
                iced::widget::button::Status::Pressed => {
                    ext.background.strong.color.scale_alpha(0.40)
                }
                _ => ext.background.strong.color.scale_alpha(0.22),
            };
            iced::widget::button::Style {
                background: Some(Background::Color(bg)),
                border: Border {
                    width: 1.0,
                    color: ext.background.strong.color.scale_alpha(0.55),
                    radius: 10.0.into(),
                },
                text_color: theme.palette().text,
                ..Default::default()
            }
        });

    let confirm_button = button(text("确认回滚"))
        .on_press(Message::Git(message::GitMessage::DiscardFile(file.clone())))
        .padding([6, 12])
        .style(|theme: &iced::Theme, status| {
            let ext = theme.extended_palette();
            let bg = match status {
                iced::widget::button::Status::Hovered => ext.danger.base.color.scale_alpha(0.92),
                iced::widget::button::Status::Pressed => ext.danger.base.color.scale_alpha(0.82),
                _ => ext.danger.base.color,
            };
            iced::widget::button::Style {
                background: Some(Background::Color(bg)),
                border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 10.0.into() },
                text_color: Color::WHITE,
                ..Default::default()
            }
        });

    let modal_actions =
        row![cancel_button, confirm_button].spacing(10).align_y(iced::Alignment::Center);

    let modal = container(
        column![text(format!("确认回滚文件：{file}？")).size(16), modal_actions].spacing(12),
    )
    .padding(16)
    .width(Length::Fixed(420.0))
    .style(|theme: &iced::Theme| iced::widget::container::Style {
        background: Some(theme.extended_palette().background.base.color.into()),
        border: Border {
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
            radius: 12.0.into(),
        },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(0.25),
            offset: iced::Vector::new(0.0, 8.0),
            blur_radius: 18.0,
        },
        ..Default::default()
    });

    let overlay = container(Space::new().width(Length::Fill).height(Length::Fill)).style(|_| {
        iced::widget::container::Style {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.45))),
            ..Default::default()
        }
    });

    let modal_layer: Element<'_, Message> = iced::widget::opaque(
        container(modal)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    );

    Some(iced::widget::stack![overlay, modal_layer].into())
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn git_repo_path_for_app(app: &App) -> Option<String> {
    ops::git_repo_path_for_app(app)
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn git_repo_path_for_app(_app: &App) -> Option<String> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn get_diff_file_metas_for_repo_path(path: &str) -> Vec<DiffFileMeta> {
    ops::get_diff_file_metas_for_repo_path(path)
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn get_diff_file_metas_for_repo_path(_path: &str) -> Vec<DiffFileMeta> {
    vec![]
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_diff_content_for_repo_path(path: &str, meta: &DiffFileMeta) -> (String, String) {
    ops::load_diff_content_for_repo_path(path, meta)
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn load_diff_content_for_repo_path(
    _path: &str,
    _meta: &DiffFileMeta,
) -> (String, String) {
    (String::new(), String::new())
}

/// 获取指定仓库路径下的所有变更文件路径
///
/// # 参数
///
/// * `repo_path` - Git 仓库的根目录路径
///
/// # 返回值
///
/// 返回变更文件的相对路径列表
///
/// # 示例
///
/// ```ignore
/// let files = changed_files_in_repo("/path/to/repo");
/// // 返回: vec!["src/main.rs", "docs/readme.md"]
/// ```
pub fn changed_files_in_repo(repo_path: &str) -> Vec<String> {
    ops::get_changed_file_paths(repo_path)
}

/// 获取当前应用项目中的所有变更文件路径
///
/// # 参数
///
/// * `app` - 应用状态引用
///
/// # 返回值
///
/// 返回变更文件的相对路径列表。如果未设置项目路径，返回空列表。
///
/// # 示例
///
/// ```ignore
/// let files = changed_files(&app);
/// // 根据项目路径获取变更文件列表
/// ```
pub fn changed_files(app: &App) -> Vec<String> {
    let Some(path) = app.project_path.as_deref() else {
        return vec![];
    };
    ops::get_changed_file_paths(path)
}

/// 判断当前主题是否为深色主题
///
/// 通过计算背景色的 RGB 亮度值来判断：如果 R+G+B < 1.5，则认为是深色主题
///
/// # 参数
///
/// * `theme` - iced 主题引用
///
/// # 返回值
///
/// 如果是深色主题返回 `true`，否则返回 `false`
fn is_dark_theme(theme: &iced::Theme) -> bool {
    let palette = theme.palette();
    palette.background.r + palette.background.g + palette.background.b < 1.5
}

/// 获取 diff 视图的颜色配置
///
/// 根据指定的主题返回 8 种颜色，用于渲染 diff 视图：
/// 1. 背景色（默认）
/// 2. 前景色（默认）
/// 3. 新增行背景色
/// 4. 新增词背景色
/// 5. 删除行背景色
/// 6. 删除词背景色
/// 7. 标题背景色
/// 8. 标题前景色
///
/// # 参数
///
/// * `theme` - diff 主题枚举（Monokai 或 GitHub）
///
/// # 返回值
///
/// 返回包含 8 种颜色的元组
fn get_diff_colors(theme: DiffTheme) -> (Color, Color, Color, Color, Color, Color, Color, Color) {
    utils::get_diff_colors(theme)
}

const GIT_FILE_CARD_GAP: f32 = 10.0;
const GIT_FILE_HEADER_ESTIMATED_HEIGHT: f32 = 32.0;
const GIT_FILE_FOCUSED_EXTRA_HEIGHT: f32 = 12.0;
const GIT_FILE_LOADING_PLACEHOLDER_HEIGHT: f32 = 44.0;
const GIT_FILE_LINE_ESTIMATED_HEIGHT: f32 = 22.0;
const GIT_FILE_VIRTUALIZATION_BUFFER: usize = 12;
const GIT_FILE_VIRTUALIZATION_MIN_ITEMS: usize = 48;

fn sum_git_file_block_height(heights: &[f32]) -> f32 {
    if heights.is_empty() {
        0.0
    } else {
        heights.iter().sum::<f32>() + GIT_FILE_CARD_GAP * heights.len().saturating_sub(1) as f32
    }
}

fn estimate_git_file_card_height(app: &App, meta: &DiffFileMeta) -> f32 {
    let mut height = GIT_FILE_HEADER_ESTIMATED_HEIGHT;

    if app.git_focused_file.as_deref() == Some(meta.path.as_str()) {
        height += GIT_FILE_FOCUSED_EXTRA_HEIGHT;
    }

    if !app.is_diff_file_expanded(&meta.path) {
        return height;
    }

    if app.git_diff_contents_loading.contains(&meta.path) {
        return height + GIT_FILE_LOADING_PLACEHOLDER_HEIGHT;
    }

    let estimated_lines = if let Some((old_content, new_content)) =
        app.git_diff_contents.get(&meta.path)
    {
        match meta.status {
            utils::FileStatus::Added | utils::FileStatus::Untracked => {
                new_content.lines().count().max(meta.insertions).max(1)
            }
            utils::FileStatus::Deleted => old_content.lines().count().max(meta.deletions).max(1),
            utils::FileStatus::Modified | utils::FileStatus::Renamed => {
                let changed_lines = meta.insertions.saturating_add(meta.deletions);
                let expanded_context =
                    changed_lines.saturating_add(crate::app::git::DIFF_CONTEXT.saturating_mul(8));
                expanded_context
                    .max(12)
                    .min(old_content.lines().count().saturating_add(new_content.lines().count()))
            }
            utils::FileStatus::Unknown => 12,
        }
    } else {
        return height + GIT_FILE_LOADING_PLACEHOLDER_HEIGHT;
    };

    height + estimated_lines.max(1) as f32 * GIT_FILE_LINE_ESTIMATED_HEIGHT + 12.0
}

fn compute_git_file_virtual_window(app: &App, file_heights: &[f32]) -> (usize, usize, f32, f32) {
    if file_heights.is_empty() {
        return (0, 0, 0.0, 0.0);
    }

    let viewport_h = app.git_diff_scroll_viewport_h.max(0.0);
    if viewport_h <= 0.0 {
        return (0, file_heights.len(), 0.0, 0.0);
    }

    let total_height = sum_git_file_block_height(file_heights);
    let max_scroll = (total_height - viewport_h).max(0.0);
    let scroll_top = app.git_diff_scroll_offset_y.clamp(0.0, 1.0) * max_scroll;
    let visible_top = scroll_top;
    let visible_bottom = (scroll_top + viewport_h).min(total_height);

    let mut start = 0usize;
    let mut cursor = 0.0f32;
    while start < file_heights.len() {
        let item_bottom = cursor + file_heights[start];
        if item_bottom >= visible_top {
            break;
        }
        cursor = item_bottom + GIT_FILE_CARD_GAP;
        start += 1;
    }

    let mut end = start;
    let mut render_cursor = cursor;
    while end < file_heights.len() {
        let item_bottom = render_cursor + file_heights[end];
        end += 1;
        if item_bottom >= visible_bottom {
            break;
        }
        render_cursor = item_bottom + GIT_FILE_CARD_GAP;
    }

    let start = start.saturating_sub(GIT_FILE_VIRTUALIZATION_BUFFER);
    let end = (end + GIT_FILE_VIRTUALIZATION_BUFFER).min(file_heights.len());
    let end = end.max(start.saturating_add(1)).min(file_heights.len());
    let top_spacer = sum_git_file_block_height(&file_heights[..start]);
    let bottom_spacer = sum_git_file_block_height(&file_heights[end..]);

    (start, end, top_spacer, bottom_spacer)
}

pub fn embedded_custom_text_diff_view(
    app: &App,
    title: String,
    file: Option<String>,
    before: String,
    after: String,
    close_message: Option<Message>,
) -> Element<'_, Message> {
    let effective_theme =
        if is_dark_theme(&app.app_theme) { DiffTheme::Monokai } else { DiffTheme::GitHub };
    let (
        bg_default,
        _fg_default,
        add_line_bg,
        add_word_bg,
        del_line_bg,
        del_word_bg,
        _header_bg,
        _header_fg,
    ) = get_diff_colors(effective_theme);

    diff_view::view_custom_text_diff(
        app,
        title,
        file,
        before,
        after,
        close_message,
        effective_theme,
        bg_default,
        add_line_bg,
        add_word_bg,
        del_line_bg,
        del_word_bg,
    )
}

/// 渲染 Git 面板的主视图
///
/// 该函数是 Git 面板的核心渲染入口，负责构建完整的 Git 变更视图，包括：
/// - 变更文件列表及其 diff 展示
/// - 文件过滤选项面板
/// - 自定义文本对比模态框
/// - 代码复制/编辑模态框
/// - 行内评论模态框
///
/// # 参数
///
/// * `app` - 应用状态引用，包含所有必要的配置和数据
///
/// # 返回值
///
/// 返回构建好的 iced `Element`，可直接嵌入到应用界面中
///
/// # 主要处理流程
///
/// 1. 收集所有已暂存的文件（包括文件级、hunk 级、行级选择）
/// 2. 根据过滤条件筛选变更文件
/// 3. 渲染头部（header）和左侧面板（可选）
/// 4. 渲染变更文件列表及其 diff
/// 5. 根据状态渲染各种模态框（复制、评论等）
pub fn view(app: &App) -> Element<'_, Message> {
    // 获取所有变更文件的元数据
    let diff_files_all = &app.git_diff_file_metas;

    // 收集所有已暂存选择的文件路径到集合中，用于后续过滤
    let mut included_files: HashSet<String> = HashSet::new();
    for f in &app.staged_files_selected {
        included_files.insert(f.clone());
    }
    for (f, _) in &app.staged_hunks_selected {
        included_files.insert(f.clone());
    }
    for (f, _) in &app.staged_lines_selected {
        included_files.insert(f.clone());
    }
    for (f, _) in &app.staged_old_lines_selected {
        included_files.insert(f.clone());
    }

    let render_ctx = diff_view::DiffRenderCtx::new(app);
    let mut diff_files = diff_files_all.clone();

    // 统计各类文件数量，用于过滤选项面板显示
    let included_count = diff_files_all.iter().filter(|m| included_files.contains(&m.path)).count();
    let excluded_count = diff_files_all.len().saturating_sub(included_count);
    // 统计新增文件数量（包括新添加和未跟踪文件）
    let new_count = diff_files_all
        .iter()
        .filter(|m| matches!(m.status, utils::FileStatus::Added | utils::FileStatus::Untracked))
        .count();
    // 统计修改文件数量（包括修改和重命名文件）
    let modified_count = diff_files_all
        .iter()
        .filter(|m| matches!(m.status, utils::FileStatus::Modified | utils::FileStatus::Renamed))
        .count();
    // 统计删除文件数量
    let deleted_count =
        diff_files_all.iter().filter(|m| matches!(m.status, utils::FileStatus::Deleted)).count();

    // 应用搜索关键词过滤
    if !app.git_filter_query.trim().is_empty() {
        let q = app.git_filter_query.to_lowercase();
        diff_files.retain(|m| m.path.to_lowercase().contains(&q));
    }

    // 应用已暂存/未暂存过滤（互斥）
    let inc = app.git_filter_included;
    let exc = app.git_filter_excluded;
    if inc ^ exc {
        let want_included = inc;
        diff_files.retain(|m| included_files.contains(&m.path) == want_included);
    }

    // 应用状态过滤（新增/修改/删除，可多选）
    let status_filters_active =
        app.git_filter_new || app.git_filter_modified || app.git_filter_deleted;
    if status_filters_active {
        diff_files.retain(|m| {
            (app.git_filter_new
                && matches!(m.status, utils::FileStatus::Added | utils::FileStatus::Untracked))
                || (app.git_filter_modified
                    && matches!(m.status, utils::FileStatus::Modified | utils::FileStatus::Renamed))
                || (app.git_filter_deleted && matches!(m.status, utils::FileStatus::Deleted))
        });
    }
    let files_list: Vec<String> = diff_files.iter().map(|m| m.path.clone()).collect();

    // 渲染头部组件
    let header = header::view(app, files_list.clone());

    // 根据当前主题确定 diff 配色方案
    let effective_theme =
        if is_dark_theme(&app.app_theme) { DiffTheme::Monokai } else { DiffTheme::GitHub };
    let (
        bg_default,
        _fg_default,
        add_line_bg,
        add_word_bg,
        del_line_bg,
        del_word_bg,
        _header_bg,
        _header_fg,
    ) = get_diff_colors(effective_theme);

    // 构建主列表容器
    let mut list = column![].spacing(10).width(Length::Fill);

    // 如果显示过滤选项，添加过滤面板
    if app.show_git_filter_options {
        let options = filter_options::view(
            app,
            included_count,
            excluded_count,
            new_count,
            modified_count,
            deleted_count,
        );
        list = list.push(options);
    }

    // 渲染自定义 diff 模态框（如果激活）
    if app.show_git_custom_diff_modal {
        let before = app.git_custom_diff_before_editor.text();
        let after = app.git_custom_diff_after_editor.text();
        let title = app.git_custom_diff_title.clone();
        let diff_view = embedded_custom_text_diff_view(app, title, None, before, after, None);
        let diff_view = container(diff_view)
            .width(Length::Fill)
            .height(Length::Fixed(360.0))
            .style(container::rounded_box)
            .padding(6);

        // 根据是否隐藏输入区域，渲染不同的头部和编辑器布局
        let (header, editors) = if app.git_custom_diff_hide_inputs {
            // 隐藏输入模式：仅显示标题和关闭按钮
            let header = row![
                text(&app.git_custom_diff_title)
                    .size(16)
                    .line_height(iced::widget::text::LineHeight::Relative(1.0)),
                container(Space::new()).width(Length::Fill),
                settings_close_button(Message::Git(message::GitMessage::CloseCustomDiffModal)),
            ]
            .align_y(iced::Alignment::Center)
            .spacing(10);
            (header, None)
        } else {
            // 显示输入模式：包含标题输入框、交换按钮、关闭按钮
            let title_input = text_input("标题", &app.git_custom_diff_title)
                .on_input(|v| Message::Git(message::GitMessage::CustomDiffTitleChanged(v)))
                .padding([6, 8])
                .size(13)
                .width(Length::Fill)
                .style(settings_text_input_style);

            let header = row![
                text("自定义对比")
                    .size(16)
                    .line_height(iced::widget::text::LineHeight::Relative(1.0)),
                title_input,
                button(text("交换").line_height(iced::widget::text::LineHeight::Relative(1.0)))
                    .on_press(Message::Git(message::GitMessage::CustomDiffSwap))
                    .padding([6, 10]),
                button(text("关闭").line_height(iced::widget::text::LineHeight::Relative(1.0)))
                    .on_press(Message::Git(message::GitMessage::CloseCustomDiffModal))
                    .padding([6, 10]),
            ]
            .align_y(iced::Alignment::Center)
            .spacing(10);

            // 左侧旧文本编辑器
            let before_editor = text_editor(&app.git_custom_diff_before_editor)
                .placeholder("旧文本…")
                .on_action(|a| Message::Git(message::GitMessage::CustomDiffBeforeEditorAction(a)))
                .height(Length::Fixed(140.0))
                .padding(10)
                .font(iced::Font::with_name("JetBrains Mono"))
                .style(settings_text_editor_style);

            // 右侧新文本编辑器
            let after_editor = text_editor(&app.git_custom_diff_after_editor)
                .placeholder("新文本…")
                .on_action(|a| Message::Git(message::GitMessage::CustomDiffAfterEditorAction(a)))
                .height(Length::Fixed(140.0))
                .padding(10)
                .font(iced::Font::with_name("JetBrains Mono"))
                .style(settings_text_editor_style);

            let editors = row![
                container(before_editor)
                    .width(Length::FillPortion(1))
                    .style(container::rounded_box),
                container(after_editor).width(Length::FillPortion(1)).style(container::rounded_box),
            ]
            .spacing(10);

            (header, Some(editors))
        };

        // 组合头部、编辑器和 diff 视图
        let block = if let Some(editors) = editors {
            column![header, editors, diff_view].spacing(12).width(Length::Fill)
        } else {
            column![header, diff_view].spacing(12).width(Length::Fill)
        };

        list = list
            .push(container(block).width(Length::Fill).style(container::rounded_box).padding(12));
    }

    // 渲染聊天文本 diff 视图（如果有）
    if let Some(d) = app.chat_text_diff.as_ref() {
        let elem = embedded_custom_text_diff_view(
            app,
            d.title.clone(),
            Some(d.file.clone()),
            d.before.clone(),
            d.after.clone(),
            Some(Message::Git(message::GitMessage::CloseChatTextDiff)),
        );
        list = list.push(elem);
    }

    let renderable_diff_files: Vec<&DiffFileMeta> =
        diff_files.iter().filter(|m| m.insertions > 0 || m.deletions > 0).collect();

    if renderable_diff_files.is_empty() && app.chat_text_diff.is_none() {
        list = list.push(text("无变更"));
    }

    if !renderable_diff_files.is_empty() {
        let use_virtualization = renderable_diff_files.len() >= GIT_FILE_VIRTUALIZATION_MIN_ITEMS
            && !app.show_git_custom_diff_modal
            && app.chat_text_diff.is_none();
        let file_heights = if use_virtualization {
            {
                renderable_diff_files
                    .iter()
                    .map(|meta| estimate_git_file_card_height(app, meta))
                    .collect::<Vec<_>>()
            }
        } else {
            Default::default()
        };
        let (start_idx, end_idx, top_spacer_h, bottom_spacer_h) = if use_virtualization {
            compute_git_file_virtual_window(app, &file_heights)
        } else {
            (0, renderable_diff_files.len(), 0.0, 0.0)
        };

        let mut file_cards = column![].spacing(GIT_FILE_CARD_GAP).width(Length::Fill);
        if top_spacer_h > 0.0 {
            file_cards = file_cards.push(Space::new().height(Length::Fixed(top_spacer_h)));
        }

        for m in
            renderable_diff_files.iter().skip(start_idx).take(end_idx.saturating_sub(start_idx))
        {
            let file = m.path.clone();
            let status = m.status;
            let insertions = m.insertions;
            let deletions = m.deletions;
            let focused = app.git_focused_file.as_deref() == Some(file.as_str());
            let is_modified =
                matches!(status, utils::FileStatus::Modified | utils::FileStatus::Renamed);
            let expanded = app.is_diff_file_expanded(&file);
            let cached_contents = app
                .git_diff_contents
                .get(&file)
                .map(|(old_content, new_content)| (old_content.as_str(), new_content.as_str()));
            let loading = expanded && app.git_diff_contents_loading.contains(&file);

            let elem = diff_view::view_file(
                app,
                &render_ctx,
                &file,
                cached_contents,
                loading,
                status,
                insertions,
                deletions,
                effective_theme,
                bg_default,
                add_line_bg,
                add_word_bg,
                del_line_bg,
                del_word_bg,
                is_modified,
            );

            let elem = if focused {
                let palette = app.app_theme.extended_palette();
                container(elem)
                    .padding(6)
                    .style(move |_| container::Style {
                        background: None,
                        border: Border {
                            color: palette.primary.base.color,
                            width: 1.0,
                            radius: 8.0.into(),
                        },
                        ..Default::default()
                    })
                    .into()
            } else {
                elem
            };

            file_cards = file_cards.push(elem);
        }

        if bottom_spacer_h > 0.0 {
            file_cards = file_cards.push(Space::new().height(Length::Fixed(bottom_spacer_h)));
        }

        list = list.push(file_cards);
    }

    // 组合头部和主列表
    let mut col = column![header].spacing(10);

    // 如果显示 diff 汇总，添加左侧面板
    if app.show_git_diff_summary {
        col = col.push(left_panel::view(app));
    }

    // 添加可滚动的主内容区域
    col = col.push(
        scrollable(container(list).width(Length::Fill).padding(iced::Padding {
            top: 0.0,
            right: 8.0,
            bottom: 0.0,
            left: 8.0,
        }))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .id(app.git_diff_scroll_id.clone())
        .on_scroll(|viewport| {
            Message::Git(message::GitMessage::DiffScrollChanged {
                offset_y: viewport.relative_offset().y,
                viewport_h: viewport.bounds().height,
            })
        })
        .height(Length::Fill),
    );

    // 包装为 MouseArea 以处理拖拽选择结束事件
    let base_content: Element<'_, Message> = iced::widget::MouseArea::new(col.height(Length::Fill))
        .on_release(Message::Git(message::GitMessage::DiffDragSelectEnd))
        .into();
    let mut base: Element<'_, Message> = base_content;

    if app.show_git_commit_help_modal {
        base = with_settings_help_modal(
            app,
            base,
            left_panel::COMMIT_HELP_TITLE,
            left_panel::COMMIT_HELP_TEXT,
            Message::Git(message::GitMessage::CommitHelpClose),
        );
    }
    if app.show_git_filter_help_modal {
        base = with_settings_help_modal(
            app,
            base,
            filter_options::FILTER_HELP_TITLE,
            filter_options::FILTER_HELP_TEXT,
            Message::Git(message::GitMessage::FilterHelpClose),
        );
    }

    // 渲染复制/编辑模态框（如果激活）
    if app.show_git_copy_modal {
        // 半透明遮罩层
        let overlay =
            container(Space::new().width(Length::Fill).height(Length::Fill)).style(|_| {
                iced::widget::container::Style {
                    background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.45))),
                    ..Default::default()
                }
            });

        // 彩色/纯文本切换开关
        let colored_toggle = iced::widget::toggler(app.git_copy_modal_use_color)
            .label("彩色")
            .on_toggle(|v| Message::Git(message::GitMessage::ToggleCopyModalColored(v)));

        // 模态框头部：标题、开关、操作按钮
        let modal_header = iced::widget::row![
            text("复制 / 编辑").size(16),
            colored_toggle,
            Space::new().width(Length::Fill),
            button(text("插入到Chat"))
                .on_press(Message::Git(message::GitMessage::InsertCopyModalToChatCurrent))
                .padding([6, 10]),
            button(text("复制"))
                .on_press(Message::Git(message::GitMessage::CopyModalCopyCurrent))
                .padding([6, 10]),
            button(text("关闭"))
                .on_press(Message::Git(message::GitMessage::CloseCopyModal))
                .padding([6, 10]),
        ]
        .align_y(iced::Alignment::Center)
        .spacing(10);

        // 根据是否使用彩色模式，渲染不同的编辑器
        let editor: Element<'_, Message> = if app.git_copy_modal_use_color {
            // 彩色模式：使用代码高亮编辑器
            let e = app
                .git_copy_modal_code_editor
                .view()
                .map(|ev| Message::Git(message::GitMessage::CopyModalCodeEditorEvent(ev)));
            container(e)
                .width(Length::Fill)
                .height(Length::Fixed(420.0))
                .style(container::rounded_box)
                .padding(6)
                .into()
        } else {
            // 纯文本模式：使用普通文本编辑器
            text_editor(&app.git_copy_modal_editor)
                .placeholder("可编辑文本；支持鼠标选择后 ⌘C 直接复制")
                .on_action(|a| Message::Git(message::GitMessage::CopyModalEditorAction(a)))
                .height(Length::Fixed(420.0))
                .padding(12)
                .font(iced::Font::with_name("Noto Sans CJK SC"))
                .style(settings_text_editor_style)
                .into()
        };

        // 组合模态框内容
        let modal = container(column![modal_header, editor].spacing(12))
            .padding(16)
            .width(Length::Fixed(860.0))
            .style(|theme: &iced::Theme| iced::widget::container::Style {
                background: Some(theme.extended_palette().background.base.color.into()),
                border: Border {
                    width: 1.0,
                    color: theme.extended_palette().background.strong.color,
                    radius: 12.0.into(),
                },
                shadow: iced::Shadow {
                    color: Color::BLACK.scale_alpha(0.25),
                    offset: iced::Vector::new(0.0, 8.0),
                    blur_radius: 18.0,
                },
                ..Default::default()
            });

        // 返回带模态框的堆叠视图
        return stack![
            base,
            overlay,
            container(modal)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
        ]
        .into();
    }

    base
}
