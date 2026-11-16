//! Git 主视图组合。宿主只提供现有子视图与只读字段投影。
use iced::{Element, Border, Length};
use iced::widget::{Space, button, container, column, row, text};
use std::collections::{HashMap, HashSet};
use crate::DiffTheme;
use super::{utils, ops::DiffFileMeta};
pub use super::panel_layout::*;
pub use super::discard_modal::{discard_file_modal, PanelMessage};
use iced::widget::{scrollable, text_editor, text_input};
use iced::widget::scrollable::{Direction, Scrollbar};
use desktop_ui_controls::system_settings_common::{settings_close_button, settings_text_editor_style, settings_text_input_style};
#[derive(Debug, Clone)]
pub enum ViewMessage<E> {
    External(E),
    Diff(super::diff_view::DiffMessage),
    CloseCustomDiffModal,
    CustomDiffAfterEditorAction(iced::widget::text_editor::Action),
    CustomDiffBeforeEditorAction(iced::widget::text_editor::Action),
    CustomDiffSwap,
    CustomDiffTitleChanged(String),
    DiffDragSelectEnd,
    DiffScrollChanged { offset_y: f32, viewport_h: f32 },
}
pub struct ViewInput<'a> {
    pub git_diff_contents: &'a HashMap<String, (String, String)>,
    pub git_diff_scroll_offset_y: f32,
    pub git_diff_scroll_viewport_h: f32,
    pub app_theme: &'a iced::Theme,
    pub desktop_locale: config_ui::DesktopLocale,
    pub expanded_files_set: &'a HashSet<String>,
    pub git_custom_diff_after_editor: &'a text_editor::Content,
    pub git_custom_diff_before_editor: &'a text_editor::Content,
    pub git_custom_diff_hide_inputs: bool,
    pub git_custom_diff_title: &'a String,
    pub git_diff_contents_loading: &'a HashSet<String>,
    pub git_diff_file_metas: &'a Vec<DiffFileMeta>,
    pub git_diff_scroll_id: &'a iced::widget::Id,
    pub git_filter_deleted: bool,
    pub git_filter_excluded: bool,
    pub git_filter_included: bool,
    pub git_filter_modified: bool,
    pub git_filter_new: bool,
    pub git_filter_query: &'a String,
    pub git_focused_file: &'a Option<String>,
    pub show_git_custom_diff_modal: bool,
    pub show_git_diff_summary: bool,
    pub show_git_filter_options: bool,
    pub staged_files_selected: &'a Vec<String>,
    pub staged_hunks_selected: &'a Vec<(String, usize)>,
    pub staged_lines_selected: &'a Vec<(String, usize)>,
    pub staged_old_lines_selected: &'a Vec<(String, usize)>,
    pub diff_state: super::diff_view::DiffViewState<'a>,
    pub render_ctx: super::diff_view::DiffRenderCtx<'a>,
    pub has_chat_diff: bool,
    pub file_menu: Option<&'a str>,
}
impl ViewInput<'_> {
    fn file_list(&self) -> FileListInput<'_> { FileListInput {
        git_focused_file: self.git_focused_file, expanded_files_set: self.expanded_files_set,
        git_diff_contents_loading: self.git_diff_contents_loading, git_diff_contents: self.git_diff_contents,
        git_diff_scroll_viewport_h: self.git_diff_scroll_viewport_h, git_diff_scroll_offset_y: self.git_diff_scroll_offset_y,
    } }
}
pub struct ViewSlots<'a, E> {
    pub header: Element<'a, ViewMessage<E>>,
    pub filter_options: Option<Element<'a, ViewMessage<E>>>,
    pub toolbar: Element<'a, ViewMessage<E>>,
    pub left_panel: Option<Element<'a, ViewMessage<E>>>,
    pub custom_diff: Option<Element<'a, ViewMessage<E>>>,
    pub chat_diff: Option<Element<'a, ViewMessage<E>>>,
}
pub struct FileProjection {
    pub diff_files: Vec<DiffFileMeta>,
    pub included_count: usize, pub excluded_count: usize,
    pub new_count: usize, pub modified_count: usize, pub deleted_count: usize,
}
pub fn project_files(app: &ViewInput<'_>) -> FileProjection {
    // 获取所有变更文件的元数据
    let diff_files_all = app.git_diff_file_metas;

    // 收集所有已暂存选择的文件路径到集合中，用于后续过滤
    let mut included_files: HashSet<String> = HashSet::new();
    for f in app.staged_files_selected {
        included_files.insert(f.clone());
    }
    for (f, _) in app.staged_hunks_selected {
        included_files.insert(f.clone());
    }
    for (f, _) in app.staged_lines_selected {
        included_files.insert(f.clone());
    }
    for (f, _) in app.staged_old_lines_selected {
        included_files.insert(f.clone());
    }


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
    FileProjection { diff_files, included_count, excluded_count, new_count, modified_count, deleted_count }
}
pub fn view<'a, E: Clone + 'static>(app: &ViewInput<'a>, projection: FileProjection, mut slots: ViewSlots<'a, E>) -> Element<'a, ViewMessage<E>> {
    let FileProjection { diff_files, .. } = projection;
    // 渲染头部组件
    let header = slots.header;

    // 根据当前主题确定 diff 配色方案
    let effective_theme =
        if is_dark_theme(app.app_theme) { DiffTheme::Monokai } else { DiffTheme::GitHub };
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
        let options = slots.filter_options.take().expect("visible filter options");
        list = list.push(options);
    }

    // 渲染自定义 diff 模态框（如果激活）
    if app.show_git_custom_diff_modal {
        let diff_view = slots.custom_diff.take().expect("custom diff slot");
        let diff_view = container(diff_view)
            .width(Length::Fill)
            .height(Length::Fixed(360.0))
            .style(container::rounded_box)
            .padding(6);

        // 根据是否隐藏输入区域，渲染不同的头部和编辑器布局
        let (header, editors) = if app.git_custom_diff_hide_inputs {
            // 隐藏输入模式：仅显示标题和关闭按钮
            let header = row![
                text(app.git_custom_diff_title)
                    .size(16)
                    .line_height(iced::widget::text::LineHeight::Relative(1.0)),
                container(Space::new()).width(Length::Fill),
                settings_close_button(ViewMessage::CloseCustomDiffModal),
            ]
            .align_y(iced::Alignment::Center)
            .spacing(10);
            (header, None)
        } else {
            // 显示输入模式：包含标题输入框、交换按钮、关闭按钮
            let title_input = text_input(
                desktop_i18n::I18n::new(app.desktop_locale).__("标题"),
                app.git_custom_diff_title,
            )
            .on_input(|v| ViewMessage::CustomDiffTitleChanged(v))
            .padding([6, 8])
            .size(13)
            .width(Length::Fill)
            .style(settings_text_input_style);

            let header = row![
                text(desktop_i18n::I18n::new(app.desktop_locale).__("自定义对比"))
                    .size(16)
                    .line_height(iced::widget::text::LineHeight::Relative(1.0)),
                title_input,
                button(
                    text(desktop_i18n::I18n::new(app.desktop_locale).__("交换"))
                        .line_height(iced::widget::text::LineHeight::Relative(1.0))
                )
                .on_press(ViewMessage::CustomDiffSwap)
                .padding([6, 10]),
                button(
                    text(desktop_i18n::I18n::new(app.desktop_locale).__("关闭"))
                        .line_height(iced::widget::text::LineHeight::Relative(1.0))
                )
                .on_press(ViewMessage::CloseCustomDiffModal)
                .padding([6, 10]),
            ]
            .align_y(iced::Alignment::Center)
            .spacing(10);

            // 左侧旧文本编辑器
            let before_editor = text_editor(app.git_custom_diff_before_editor)
                .placeholder(desktop_i18n::I18n::new(app.desktop_locale).__("旧文本…"))
                .on_action(|a| ViewMessage::CustomDiffBeforeEditorAction(a))
                .height(Length::Fixed(140.0))
                .padding(10)
                .font(iced::Font::with_name("JetBrains Mono"))
                .style(settings_text_editor_style);

            // 右侧新文本编辑器
            let after_editor = text_editor(app.git_custom_diff_after_editor)
                .placeholder(desktop_i18n::I18n::new(app.desktop_locale).__("新文本…"))
                .on_action(|a| ViewMessage::CustomDiffAfterEditorAction(a))
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
    if let Some(elem) = slots.chat_diff.take() { list = list.push(elem); }

    let renderable_diff_files: Vec<&DiffFileMeta> =
        diff_files.iter().filter(|m| m.insertions > 0 || m.deletions > 0).collect();

    if renderable_diff_files.is_empty() && !app.has_chat_diff {
        list = list.push(text(desktop_i18n::I18n::new(app.desktop_locale).__("无变更")));
    }

    if !renderable_diff_files.is_empty() {
        let use_virtualization = renderable_diff_files.len() >= GIT_FILE_VIRTUALIZATION_MIN_ITEMS
            && !app.show_git_custom_diff_modal
            && !app.has_chat_diff;
        let file_heights = if use_virtualization {
            {
                renderable_diff_files
                    .iter()
                    .map(|meta| estimate_git_file_card_height(&app.file_list(), meta))
                    .collect::<Vec<_>>()
            }
        } else {
            Default::default()
        };
        let (start_idx, end_idx, top_spacer_h, bottom_spacer_h) = if use_virtualization {
            compute_git_file_virtual_window(&app.file_list(), &file_heights)
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
            let expanded = app.expanded_files_set.contains(&file);
            let cached_contents = app
                .git_diff_contents
                .get(&file)
                .map(|(old_content, new_content)| (old_content.as_str(), new_content.as_str()));
            let loading = expanded && app.git_diff_contents_loading.contains(&file);

            let elem = super::diff_view::file_view::view_file(
                &app.diff_state, expanded, app.file_menu == Some(file.as_str()),
                &app.render_ctx,
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

            let elem = elem.map(ViewMessage::Diff);
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
    let mut col = column![header, slots.toolbar].spacing(10);

    // 如果显示 diff 汇总，添加左侧面板
    if app.show_git_diff_summary {
        col = col.push(slots.left_panel.take().expect("visible commit panel"));
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
            ViewMessage::DiffScrollChanged {
                offset_y: viewport.relative_offset().y,
                viewport_h: viewport.bounds().height,
            }
        })
        .height(Length::Fill),
    );

    // 包装为 MouseArea 以处理拖拽选择结束事件
    let base_content: Element<'_, ViewMessage<E>> = iced::widget::MouseArea::new(col.height(Length::Fill))
        .on_release(ViewMessage::DiffDragSelectEnd)
        .into();
    base_content
}
