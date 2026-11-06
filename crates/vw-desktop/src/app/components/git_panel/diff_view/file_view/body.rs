//! 工作区变更与提交历史共用的 diff 内容渲染。
use super::{
    DiffRenderCtx, FileStatus, Lang, added_deleted, gaps, hunk_ops, lang_for_file, start_end,
};
use crate::app::{App, DiffTheme, Message};
use iced::widget::{column, container, text};
use iced::{Background, Border, Color, Element, Length};

pub(in crate::app::components::git_panel::diff_view) fn view_file_body(
    app: &App,
    render_ctx: &DiffRenderCtx<'_>,
    file: &str,
    contents: Option<(&str, &str)>,
    loading: bool,
    status: FileStatus,
    effective_theme: DiffTheme,
    bg_default: Color,
    add_line_bg: Color,
    add_word_bg: Color,
    del_line_bg: Color,
    del_word_bg: Color,
    is_modified: bool,
) -> Element<'static, Message> {
    let (old_content, new_content) = contents.unwrap_or(("", ""));
    let old_lines: Vec<_> = old_content.lines().collect();
    let new_lines: Vec<_> = new_content.lines().collect();
    let diff = similar::TextDiff::from_lines(old_content, new_content);
    let allow_revert = !render_ctx.read_only;
    let mut list: Vec<Element<'static, Message>> = Vec::new();
    if loading || contents.is_none() {
        let placeholder =
            if loading { "正在加载文件内容..." } else { "文件内容暂不可用" };
        list.push(
            container(
                text(render_ctx.history_notice.clone().unwrap_or_else(|| {
                    crate::app::i18n::I18n::new(app.desktop_locale).__(placeholder).to_owned()
                }))
                .size(12),
            )
            .width(Length::Fill)
            .padding([10, 12])
            .into(),
        );
        return container(iced::widget::Column::with_children(list).spacing(0).width(Length::Fill))
            .width(Length::Fill)
            .into();
    }

    // 确定文件的语言类型（用于语法高亮）
    // 新增、未跟踪、删除的文件不进行语法高亮
    let lang = if matches!(status, FileStatus::Added | FileStatus::Untracked | FileStatus::Deleted)
    {
        Lang::Other
    } else {
        lang_for_file(file)
    };

    // 悬停高亮颜色设置
    let hover_color = Color::from_rgba8(255, 210, 0, 1.0);
    let hover_mix: f32 = 0.22;
    let hover_alpha: f32 = 0.22;
    let hover_tint = Color::from_rgba(hover_color.r, hover_color.g, hover_color.b, hover_alpha);

    // 检查是否有选中的行（影响某些交互行为）
    // ========== 处理新增文件（纯绿色显示） ==========
    if matches!(status, FileStatus::Added | FileStatus::Untracked) {
        let col = added_deleted::render_added_lines(
            app,
            render_ctx,
            file,
            &new_lines,
            lang,
            effective_theme,
            add_line_bg,
            add_word_bg,
            hover_color,
            hover_mix,
            hover_tint,
            false,
            allow_revert,
        );
        list.push(container(col).width(Length::Fill).padding([4, 0]).into());
        return container(iced::widget::Column::with_children(list).spacing(0).width(Length::Fill))
            .width(Length::Fill)
            .into();
    }

    // ========== 处理删除文件（纯红色显示） ==========
    if matches!(status, FileStatus::Deleted) {
        let col = added_deleted::render_deleted_lines(
            app,
            render_ctx,
            file,
            &old_lines,
            lang,
            effective_theme,
            del_line_bg,
            hover_color,
            hover_mix,
            hover_tint,
            false,
            allow_revert,
        );
        list.push(container(col).width(Length::Fill).padding([4, 0]).into());
        return container(iced::widget::Column::with_children(list).spacing(0).width(Length::Fill))
            .width(Length::Fill)
            .into();
    }

    // ========== 处理修改/重命名文件（标准差异视图） ==========
    let groups = diff.grouped_ops(crate::app::git::DIFF_CONTEXT);

    // 检查是否有手动展开的差异块
    let has_hunk_manual_expanded = app.expanded_hunks.iter().any(|(f, _)| f.as_str() == file);

    // 自动展开预算：小于等于这个行数的差异块会自动展开
    let mut auto_expanded_budget: usize = 500;

    // 追踪上一个差异块结束的行号（用于计算间隙区域）
    let mut last_old_end = 0;
    let mut last_new_end = 0;

    // 遍历每个差异块（hunk）并渲染
    for (i, group) in groups.iter().enumerate() {
        // 估算当前差异块的行数（用于决定是否自动展开）
        let group_line_estimate = group.iter().fold(0usize, |acc, op| match op {
            similar::DiffOp::Equal { len, .. } => acc + len,
            similar::DiffOp::Delete { old_len, .. } => acc + old_len,
            similar::DiffOp::Insert { new_len, .. } => acc + new_len,
            similar::DiffOp::Replace { old_len, new_len, .. } => acc + old_len + new_len,
        });

        // 判断差异块是否展开
        // 展开条件：手动展开 或 （没有手动展开过且行数在预算内）
        let hunk_expanded =
            app.expanded_hunks.iter().any(|(f, idx)| f.as_str() == file && *idx == i)
                || (!has_hunk_manual_expanded && group_line_estimate <= auto_expanded_budget && {
                    auto_expanded_budget = auto_expanded_budget.saturating_sub(group_line_estimate);
                    true
                });

        // 计算当前差异块在旧文件和新文件中的范围
        let mut current_old_start = usize::MAX;
        let mut current_old_end = last_old_end;
        let mut current_new_end = last_new_end;

        // 遍历差异块中的所有操作，确定行号范围
        for op in group.iter() {
            match op {
                similar::DiffOp::Equal { old_index, new_index, len } => {
                    current_old_start = current_old_start.min(*old_index);
                    current_old_end = current_old_end.max(old_index + *len);
                    current_new_end = current_new_end.max(new_index + *len);
                }
                similar::DiffOp::Delete { old_index, old_len, new_index } => {
                    current_old_start = current_old_start.min(*old_index);
                    current_old_end = current_old_end.max(old_index + *old_len);
                    current_new_end = current_new_end.max(*new_index);
                }
                similar::DiffOp::Insert { new_index, new_len, .. } => {
                    current_new_end = current_new_end.max(new_index + *new_len);
                }
                similar::DiffOp::Replace { old_index, old_len, new_index, new_len } => {
                    current_old_start = current_old_start.min(*old_index);
                    current_old_end = current_old_end.max(old_index + *old_len);
                    current_new_end = current_new_end.max(new_index + *new_len);
                }
            }
        }

        // 如果没有找到起始位置（例如只有插入操作），使用上一个块的结束位置
        if current_old_start == usize::MAX {
            current_old_start = last_old_end;
        }
        // 渲染差异块之间的间隙区域（折叠的内容）
        let gap_elems = gaps::render_gap(
            app,
            render_ctx,
            file,
            &old_lines,
            start_end::GapRange {
                start_old: last_old_end,
                end_old: current_old_start,
                start_new: last_new_end,
                gap_id: i * 2,
            },
            lang,
            effective_theme,
            bg_default,
            hover_tint,
            false,
        );
        for elem in gap_elems {
            list.push(elem);
        }

        // 创建差异块列容器
        let mut col = column![].spacing(0);

        // 如果差异块未展开，显示折叠摘要
        if !hunk_expanded {
            col = col.push(
                container(
                    text(
                        crate::app::i18n::I18n::new(app.desktop_locale)
                            .format("已折叠 %u 行差异", &[group_line_estimate.into()])
                            .expect("valid folded count"),
                    )
                    .size(12)
                    .style(|theme: &iced::Theme| iced::widget::text::Style {
                        color: Some(theme.extended_palette().secondary.base.text.scale_alpha(0.80)),
                    }),
                )
                .width(Length::Fill)
                .padding([6, 10])
                .style(|theme: &iced::Theme| {
                    let is_dark = theme.palette().background.r
                        + theme.palette().background.g
                        + theme.palette().background.b
                        < 1.5;
                    let ext = theme.extended_palette();
                    iced::widget::container::Style {
                        background: Some(Background::Color(if is_dark {
                            ext.background.weak.color.scale_alpha(0.40)
                        } else {
                            ext.background.weak.color.scale_alpha(0.88)
                        })),
                        border: Border {
                            width: 1.0,
                            color: ext.background.strong.color.scale_alpha(if is_dark {
                                0.28
                            } else {
                                0.14
                            }),
                            radius: 8.0.into(),
                        },
                        ..Default::default()
                    }
                }),
            );
            list.push(col.into());

            // 更新最后一个差异块的结束位置
            last_old_end = current_old_end;
            last_new_end = current_new_end;
            continue;
        }

        // 差异块已展开，渲染详细的差异操作
        let hunk_col = hunk_ops::render_hunk_ops(
            app,
            render_ctx,
            file,
            group,
            i,
            &old_lines,
            &new_lines,
            lang,
            effective_theme,
            bg_default,
            add_line_bg,
            add_word_bg,
            del_line_bg,
            del_word_bg,
            hover_color,
            hover_mix,
            hover_tint,
            false,
            is_modified,
        );

        // 将差异操作行添加到列容器
        for elem in hunk_col {
            col = col.push(elem);
        }

        // 更新最后一个差异块的结束位置
        last_old_end = current_old_end;
        last_new_end = current_new_end;

        list.push(container(col).padding([4, 0]).into());
    }

    // 渲染最后一个差异块之后到文件末尾的间隙区域
    let gap_elems = gaps::render_gap(
        app,
        render_ctx,
        file,
        &old_lines,
        start_end::GapRange {
            start_old: last_old_end,
            end_old: old_lines.len(),
            start_new: last_new_end,
            gap_id: groups.len(),
        },
        lang,
        effective_theme,
        bg_default,
        hover_tint,
        false,
    );
    for elem in gap_elems {
        list.push(elem);
    }

    container(iced::widget::Column::with_children(list).spacing(0)).width(Length::Fill).into()
}
