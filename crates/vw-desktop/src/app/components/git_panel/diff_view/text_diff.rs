use iced::widget::{MouseArea, column, container, row, text};
use iced::{Background, Border, Color, Element, Length};
use similar::{ChangeTag, TextDiff};

use crate::app::{App, DiffTheme, Message, message};

use super::super::utils::{lang_for_file, render_line_content};
use super::{
    DiffRenderCtx, DiffSplitPaneTone, diff_highlight_enabled, diff_split_divider, diff_split_pane,
    diff_split_pane_with_background, diff_split_row, header, markers, selection,
    split_line_number_area, wrap_diff_row_with_context_menu,
};

/// 创建自定义文本差异对比视图
///
/// 该函数生成一个完整的差异对比 UI 组件，用于显示两个文本版本之间的差异。
/// 根据应用设置，可以选择合并视图或分离视图两种显示模式。
///
/// # 参数
///
/// * `app` - 应用状态引用，用于访问用户设置和选择状态
/// * `title` - 差异对比的标题，通常为文件名或描述性标题
/// * `file` - 可选的文件路径，用于确定语法高亮语言
/// * `old_content` - 旧版本文本内容（修改前）
/// * `new_content` - 新版本文本内容（修改后）
/// * `close_message` - 可选的关闭按钮消息，点击关闭按钮时发送
/// * `effective_theme` - 代码高亮的主题（浅色/深色）
/// * `bg_default` - 默认背景颜色（用于未更改的行）
/// * `add_line_bg` - 增加行的背景颜色
/// * `add_word_bg` - 增加行中字符级高亮的背景颜色
/// * `del_line_bg` - 删除行的背景颜色
/// * `del_word_bg` - 删除行中字符级高亮的背景颜色
///
/// # 返回值
///
/// 返回一个 `Element`，包含完整的差异对比视图，包括：
/// - 头部：显示标题、插入/删除统计信息和关闭按钮
/// - 内容：逐行显示差异，带有行号、差异标记和交互功能
pub fn view_custom_text_diff(
    app: &App,
    title: String,
    file: Option<String>,
    old_content: String,
    new_content: String,
    close_message: Option<Message>,
    effective_theme: DiffTheme,
    bg_default: Color,
    add_line_bg: Color,
    add_word_bg: Color,
    del_line_bg: Color,
    del_word_bg: Color,
) -> Element<'_, Message> {
    let render_ctx = DiffRenderCtx::new(app);
    let file_key = file.unwrap_or_else(|| title.clone());
    let lang = lang_for_file(&file_key);
    let diff = TextDiff::from_lines(&old_content, &new_content);

    let mut old_line: usize = 0;
    let mut new_line: usize = 0;
    let mut insertions: usize = 0;
    let mut deletions: usize = 0;

    let hover_color = Color::from_rgba8(255, 210, 0, 1.0);
    let hover_alpha: f32 = 0.22;
    let hover_mix: f32 = 0.22;
    let hover_tint = Color::from_rgba(hover_color.r, hover_color.g, hover_color.b, hover_alpha);

    let mut list = column![].spacing(0);

    for change in diff.iter_all_changes() {
        let tag = change.tag();

        match tag {
            ChangeTag::Insert => insertions = insertions.saturating_add(1),
            ChangeTag::Delete => deletions = deletions.saturating_add(1),
            ChangeTag::Equal => {}
        }

        let marker = match tag {
            ChangeTag::Insert => markers::LineMarkerKind::Add,
            ChangeTag::Delete => markers::LineMarkerKind::Delete,
            ChangeTag::Equal => markers::LineMarkerKind::None,
        };

        let old_idx = if matches!(tag, ChangeTag::Insert) { None } else { Some(old_line) };
        let new_idx = if matches!(tag, ChangeTag::Delete) { None } else { Some(new_line) };

        let raw = change.to_string();
        let content = raw.strip_suffix('\n').unwrap_or(raw.as_str());

        let event_is_old = matches!(tag, ChangeTag::Delete);
        let event_line = if event_is_old { old_idx.unwrap_or(0) } else { new_idx.unwrap_or(0) };
        let hovered = selection::is_diff_hovered(app, &file_key, event_line, event_is_old);

        let (line_bg, word_bg) = match tag {
            ChangeTag::Insert => {
                let lb = if hovered {
                    selection::mix_color(add_line_bg, hover_color, hover_mix)
                } else {
                    add_line_bg
                };
                let wb = if hovered {
                    selection::mix_color(add_word_bg, hover_color, hover_mix)
                } else {
                    add_word_bg
                };
                (lb, wb)
            }
            ChangeTag::Delete => {
                let lb = if hovered {
                    selection::mix_color(del_line_bg, hover_color, hover_mix)
                } else {
                    del_line_bg
                };
                let wb = if hovered {
                    selection::mix_color(del_word_bg, hover_color, hover_mix)
                } else {
                    del_word_bg
                };
                (lb, wb)
            }
            ChangeTag::Equal => {
                if hovered {
                    (hover_tint, hover_tint)
                } else {
                    (bg_default, bg_default)
                }
            }
        };

        let row_element: Element<'_, Message> = if app.merge_view {
            let selected =
                selection::is_diff_selected(app, &render_ctx, &file_key, event_line, event_is_old);
            let marker_emphasis = hovered || selected;

            let line_num_area: Element<'_, Message> = if let Some((i, is_old, tone)) = old_idx
                .filter(|_| matches!(tag, ChangeTag::Delete))
                .map(|i| (i, true, markers::LineNumberTone::Delete))
                .or_else(|| {
                    new_idx.map(|i| {
                        (
                            i,
                            false,
                            if matches!(tag, ChangeTag::Insert) {
                                markers::LineNumberTone::Add
                            } else {
                                markers::LineNumberTone::Neutral
                            },
                        )
                    })
                }) {
                MouseArea::new(markers::line_number_cell_with_tone((i + 1).to_string(), tone))
                    .on_press(Message::Git(message::GitMessage::DiffDragSelectStart(
                        file_key.clone(),
                        i,
                        is_old,
                        content.to_string(),
                    )))
                    .on_enter(Message::Git(message::GitMessage::DiffDragSelectHover(
                        file_key.clone(),
                        i,
                        is_old,
                    )))
                    .into()
            } else {
                markers::empty_line_number_cell()
            };

            let content_row = render_line_content(
                content,
                lang,
                effective_theme,
                diff_highlight_enabled(app),
                &[],
                line_bg,
                word_bg,
            );

            let content_area: Element<'_, Message> =
                MouseArea::new(container(content_row).width(Length::Fill).padding([0, 2]))
                    .on_press(Message::Git(message::GitMessage::DiffDragSelectStart(
                        file_key.clone(),
                        event_line,
                        event_is_old,
                        content.to_string(),
                    )))
                    .on_enter(Message::Git(message::GitMessage::DiffDragSelectHover(
                        file_key.clone(),
                        event_line,
                        event_is_old,
                    )))
                    .into();

            let row = container(
                row![
                    markers::line_marker_cell_emphasis(marker, marker_emphasis),
                    super::file_view::diff_line_select_spacer(),
                    line_num_area,
                    content_area
                ]
                .spacing(4)
                .align_y(iced::Alignment::Center),
            )
            .width(Length::Fill)
            .style(move |_| iced::widget::container::Style {
                background: Some(Background::Color(line_bg)),
                border: if selected { selection::selected_border() } else { Border::default() },
                ..Default::default()
            });

            let wrapped = MouseArea::new(row)
                .on_enter(Message::Git(message::GitMessage::DiffHoverEnter(
                    file_key.clone(),
                    event_line,
                    event_is_old,
                )))
                .on_exit(Message::Git(message::GitMessage::DiffHoverExit(
                    file_key.clone(),
                    event_line,
                    event_is_old,
                )))
                .into();

            wrap_diff_row_with_context_menu(
                app,
                &file_key,
                event_line,
                event_is_old,
                content.to_string(),
                wrapped,
            )
        } else {
            let left_selected = old_idx
                .is_some_and(|i| selection::is_diff_selected(app, &render_ctx, &file_key, i, true));
            let right_selected = new_idx.is_some_and(|i| {
                selection::is_diff_selected(app, &render_ctx, &file_key, i, false)
            });
            let pane_emphasis = hovered || left_selected || right_selected;

            let left_num_area = split_line_number_area(
                &file_key,
                old_idx.map(|i| (i, true)),
                content,
                match tag {
                    ChangeTag::Delete => markers::LineNumberTone::Delete,
                    ChangeTag::Equal | ChangeTag::Insert => markers::LineNumberTone::Neutral,
                },
            );
            let right_num_area = split_line_number_area(
                &file_key,
                new_idx.map(|i| (i, false)),
                content,
                match tag {
                    ChangeTag::Insert => markers::LineNumberTone::Add,
                    ChangeTag::Equal | ChangeTag::Delete => markers::LineNumberTone::Neutral,
                },
            );

            let left_part: Element<'_, Message> = if let Some(i) = old_idx {
                let content_cell: Element<'_, Message> =
                    if matches!(tag, ChangeTag::Delete) || matches!(tag, ChangeTag::Equal) {
                        let row = render_line_content(
                            content,
                            lang,
                            effective_theme,
                            diff_highlight_enabled(app),
                            &[],
                            line_bg,
                            word_bg,
                        );
                        MouseArea::new(container(row).width(Length::Fill).padding([0, 2]))
                            .on_press(Message::Git(message::GitMessage::DiffDragSelectStart(
                                file_key.clone(),
                                i,
                                true,
                                content.to_string(),
                            )))
                            .on_enter(Message::Git(message::GitMessage::DiffDragSelectHover(
                                file_key.clone(),
                                i,
                                true,
                            )))
                            .into()
                    } else {
                        container(text("")).width(Length::Fill).into()
                    };

                let left_pane_padding =
                    if matches!(tag, ChangeTag::Delete) { [0, 1] } else { [0, 2] };

                diff_split_pane_with_background(
                    container(
                        row![left_num_area, content_cell]
                            .spacing(0)
                            .align_y(iced::Alignment::Center)
                            .width(Length::Fill),
                    )
                    .padding(left_pane_padding)
                    .into(),
                    line_bg,
                    pane_emphasis,
                )
            } else {
                diff_split_pane(
                    container(
                        row![left_num_area, container(text("")).width(Length::Fill)]
                            .width(Length::Fill),
                    )
                    .padding([0, 2])
                    .into(),
                    DiffSplitPaneTone::Empty,
                    pane_emphasis,
                )
            };

            let right_part: Element<'_, Message> = if let Some(i) = new_idx {
                let content_cell: Element<'_, Message> =
                    if matches!(tag, ChangeTag::Insert) || matches!(tag, ChangeTag::Equal) {
                        let row = render_line_content(
                            content,
                            lang,
                            effective_theme,
                            diff_highlight_enabled(app),
                            &[],
                            line_bg,
                            word_bg,
                        );
                        MouseArea::new(container(row).width(Length::Fill).padding([0, 2]))
                            .on_press(Message::Git(message::GitMessage::DiffDragSelectStart(
                                file_key.clone(),
                                i,
                                false,
                                content.to_string(),
                            )))
                            .on_enter(Message::Git(message::GitMessage::DiffDragSelectHover(
                                file_key.clone(),
                                i,
                                false,
                            )))
                            .into()
                    } else {
                        container(text("")).width(Length::Fill).into()
                    };

                diff_split_pane_with_background(
                    container(
                        row![right_num_area, content_cell]
                            .spacing(0)
                            .align_y(iced::Alignment::Center)
                            .width(Length::Fill),
                    )
                    .padding([0, 2])
                    .into(),
                    line_bg,
                    pane_emphasis,
                )
            } else {
                diff_split_pane(
                    container(
                        row![right_num_area, container(text("")).width(Length::Fill)]
                            .width(Length::Fill),
                    )
                    .padding([0, 2])
                    .into(),
                    DiffSplitPaneTone::Empty,
                    pane_emphasis,
                )
            };

            let row = container(diff_split_row([
                markers::line_marker_cell_emphasis(marker, pane_emphasis),
                left_part,
                diff_split_divider(),
                right_part,
            ]))
            .width(Length::Fill)
            .padding([0, 1])
            .style(move |_| iced::widget::container::Style {
                background: Some(Background::Color(line_bg)),
                ..Default::default()
            });

            let wrapped = MouseArea::new(row)
                .on_enter(Message::Git(message::GitMessage::DiffHoverEnter(
                    file_key.clone(),
                    event_line,
                    event_is_old,
                )))
                .on_exit(Message::Git(message::GitMessage::DiffHoverExit(
                    file_key.clone(),
                    event_line,
                    event_is_old,
                )))
                .into();

            wrap_diff_row_with_context_menu(
                app,
                &file_key,
                event_line,
                event_is_old,
                content.to_string(),
                wrapped,
            )
        };

        list = list.push(row_element);

        if old_idx.is_some() {
            old_line = old_line.saturating_add(1);
        }
        if new_idx.is_some() {
            new_line = new_line.saturating_add(1);
        }
    }

    let header_content =
        header::build_diff_header(title, insertions, deletions, close_message, None, None, None);
    let header = header::wrap_diff_header(header_content, insertions, deletions);

    container(column![header, container(list).padding([0, 0])].spacing(0)).into()
}
