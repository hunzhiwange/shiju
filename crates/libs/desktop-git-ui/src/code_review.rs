//! 按文件组合代码审查差异。
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Background, Color, Element, Length};
#[derive(Debug, Clone)]
pub enum ReviewMessage { Refresh, ToggleExpandFile(String) }
pub fn build_code_review_view_from_diff(
    locale: config_ui::DesktopLocale,
    expanded_files: &std::collections::HashSet<String>,
    files: Vec<String>,
    patches: std::collections::HashMap<String, String>,
) -> Element<'static, ReviewMessage> {
    let refresh = button(text(desktop_i18n::I18n::new(locale).__("刷新")))
        .on_press(ReviewMessage::Refresh);

    let header = row![text("Git Diff"), refresh].spacing(10);

    let mut list = column![header].spacing(10);
    if files.is_empty() {
        list = list.push(text(desktop_i18n::I18n::new(locale).__("无变更")));
        return scrollable(container(list).width(Length::Fill)).into();
    }

    for file in files {
        let expanded = expanded_files.contains(&file);
        let arrow = if expanded { "▼" } else { "▶" };
        let file_header = button(row![text(arrow), text(file.clone())].spacing(8))
            .padding([2, 6])
            .on_press(ReviewMessage::ToggleExpandFile(file.clone()))
            .width(Length::Fill);

        list = list.push(file_header);

        if expanded {
            let patch = patches.get(&file).cloned().unwrap_or_default();
            let body = render_unified_patch(patch);
            list = list.push(container(body).padding([6, 0]));
        }
    }

    scrollable(container(list).width(Length::Fill)).into()
}

/// 渲染合并视图的 patch 内容
///
/// 将 patch 内容逐行渲染为带样式的高亮元素，
/// 所有行在单列中垂直排列。
///
/// # 参数
///
/// - `patch`: Git patch 格式的差异字符串
///
/// # 返回值
///
/// 返回包含所有渲染行的容器元素
fn render_unified_patch(patch: String) -> Element<'static, ReviewMessage> {
    let mut col = column![];
    for line in patch.lines() {
        col = col.push(render_unified_line(line.to_string()));
    }
    container(col).width(Length::Fill).into()
}

#[cfg(test)]
pub(crate) fn render_unified_patch_for_test(patch: String) -> Element<'static, ReviewMessage> {
    render_unified_patch(patch)
}

/// 渲染合并视图中的单行
///
/// 根据行内容（新增/删除/上下文）应用不同的样式：
/// - 新增行：绿色背景和边框
/// - 删除行：红色背景和边框
/// - 上下文行：默认深色背景
///
/// # 参数
///
/// - `line`: 要渲染的行内容
///
/// # 返回值
///
/// 返回带背景色和边框样式的容器元素
pub(crate) fn render_unified_line(line: String) -> Element<'static, ReviewMessage> {
    let (bg, fg) = unified_style_for_line(&line);
    let is_add = line.starts_with('+') && !line.starts_with("+++");
    let is_del = line.starts_with('-') && !line.starts_with("---");
    let border_color = if is_add {
        Color::from_rgb8(0x2E, 0xA0, 0x43)
    } else if is_del {
        Color::from_rgb8(0xF8, 0x51, 0x49)
    } else {
        Color::from_rgb8(0x30, 0x36, 0x3D)
    };
    let border_width = if is_add || is_del { 1.0 } else { 0.0 };
    let t = text(line).color(fg);
    container(t)
        .width(Length::Fill)
        .padding([2, 6])
        .style(move |_| iced::widget::container::Style {
            text_color: None,
            background: Some(Background::Color(bg)),
            border: iced::Border { radius: 0.0.into(), width: border_width, color: border_color },
            shadow: iced::Shadow::default(),
            snap: false,
        })
        .into()
}

/// 获取合并视图中行的样式
///
/// 根据行的前缀判断其类型，返回对应的背景色和前景色。
///
/// # 参数
///
/// - `line`: 要分析的行内容
///
/// # 返回值
///
/// 返回元组 `(背景色, 前景色)`
///
/// # 颜色方案
///
/// | 行类型            | 背景色 (RGB)      | 前景色 (RGB)      |
/// |------------------|------------------|------------------|
/// | 元数据行         | #0D1117          | #C9D1D9          |
/// | Hunk 标记（@@）   | #171B22          | #79C0FF          |
/// | 新增行（+）       | #103B1A          | #C0F2D0          |
/// | 删除行（-）       | #4C1F1F          | #FFC2C2          |
/// | 默认             | #0D1117          | #C9D1D9          |
pub(crate) fn unified_style_for_line(line: &str) -> (Color, Color) {
    let bg_default = Color::from_rgb8(0x0D, 0x11, 0x17);
    let fg_default = Color::from_rgb8(0xC9, 0xD1, 0xD9);

    if line.starts_with("diff --git ")
        || line.starts_with("index ")
        || line.starts_with("--- ")
        || line.starts_with("+++ ")
    {
        return (Color::from_rgb8(0x0D, 0x11, 0x17), Color::from_rgb8(0xC9, 0xD1, 0xD9));
    }
    if line.starts_with("@@") {
        return (Color::from_rgb8(0x17, 0x1B, 0x22), Color::from_rgb8(0x79, 0xC0, 0xFF));
    }
    if line.starts_with('+') && !line.starts_with("+++") {
        return (Color::from_rgb8(0x10, 0x3B, 0x1A), Color::from_rgb8(0xC0, 0xF2, 0xD0));
    }
    if line.starts_with('-') && !line.starts_with("---") {
        return (Color::from_rgb8(0x4C, 0x1F, 0x1F), Color::from_rgb8(0xFF, 0xC2, 0xC2));
    }
    (bg_default, fg_default)
}
