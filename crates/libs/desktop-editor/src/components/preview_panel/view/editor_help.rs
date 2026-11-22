//! Accurate help for the pinned editor; all text uses the normal UI font.
use super::Message;
use desktop_ui_controls::system_settings_common::rounded_action_btn_style;

use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Border, Element, Length, Theme};

pub fn entries(mac: bool, vim: bool, folding: bool) -> Vec<(&'static str, String)> {
    let command = if mac { "Cmd" } else { "Ctrl" };
    let alt = if mac { "Option" } else { "Alt" };
    let mut rows = vec![
        ("常用操作", String::new()),
        ("切换编辑模式", format!("{command}+{alt}+V")),
        ("搜索", format!("{command}+F")),
        ("替换", format!("{command}+H")),
        ("下一个 / 上一个匹配", "F3 / Shift+F3".into()),
        ("跳转到行", format!("{command}+G")),
        ("命令面板", format!("{command}+Shift+P")),
    ];
    if folding {
        rows.push(("切换折叠", "Ctrl+.".into()));
    }

    let standard = [
        ("普通模式", String::new()),
        ("移动 / 扩展选择", "↑ ↓ ← → / Shift+↑ ↓ ← →".into()),
        ("复制 / 剪切 / 粘贴", format!("{command}+C / X / V")),
        ("全选", format!("{command}+A")),
        ("撤销 / 重做", format!("{command}+Z / {command}+Shift+Z")),
        ("插入缩进 / 焦点导航", "Tab / Shift+Tab".into()),
        ("选择下一个相同项", format!("{command}+D")),
    ];
    let vim_mode = [
        ("Vim 模式", String::new()),
        ("移动光标", "h j k l / w b e / gg G".into()),
        ("进入插入模式", "i / a / o".into()),
        ("字符 / 行选择", "v / V".into()),
        ("返回普通态（Normal）", "Esc".into()),
        ("删除 / 复制 / 粘贴", "dd / yy / p".into()),
        ("搜索 / 下一个 / 上一个", "/ / n / N".into()),
        ("退出 Vim", ":q".into()),
    ];
    if vim {
        rows.extend(vim_mode);
        rows.extend(standard);
    } else {
        rows.extend(standard);
        rows.extend(vim_mode);
    }
    rows
}

pub fn view(
    editor: Option<&iced_code_editor::CodeEditor>,
    i18n: desktop_i18n::I18n,
) -> Element<'_, Message> {
    let vim = editor.is_some_and(|editor| editor.vim_enabled());
    let folding = editor.is_some_and(|editor| editor.folding_enabled());
    let mut body = iced::widget::Column::<Message>::new().spacing(8);
    for (label, key) in entries(cfg!(target_os = "macos"), vim, folding) {
        let item: Element<'_, Message> = if key.is_empty() {
            let heading = text(i18n.__(label)).size(14);
            match label {
                "普通模式" | "Vim 模式" => container(heading)
                    .width(Length::Fill)
                    .padding([4, 8])
                    .style(move |theme: &Theme| {
                        let palette = theme.extended_palette();
                        let pair = if label == "Vim 模式" {
                            palette.success.weak
                        } else {
                            palette.primary.weak
                        };
                        container::Style {
                            background: Some(pair.color.into()),
                            text_color: Some(pair.text),
                            border: Border::default().rounded(5),
                            ..Default::default()
                        }
                    })
                    .into(),
                _ => heading.into(),
            }
        } else {
            let shortcut = container(text(key).size(12)).padding([3, 6]).style(|theme: &Theme| {
                let palette = theme.extended_palette();
                let pair = palette.background.weak;
                container::Style {
                    background: Some(pair.color.into()),
                    text_color: Some(pair.text),
                    border: Border::default().rounded(4),
                    ..Default::default()
                }
            });
            row![
                text(i18n.__(label)).size(12).width(Length::Fill),
                container(shortcut).width(Length::Fill),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .into()
        };
        body = body.push(item);
    }
    column![
        row![
            text(i18n.__("编辑器使用帮助")).size(14),
            Space::new().width(Length::Fill),
            button(text(i18n.__("关闭")))
                .padding([4, 8])
                .style(rounded_action_btn_style)
                .on_press(Message::ClosePopover)
        ]
        .align_y(iced::Alignment::Center),
        scrollable(container(body).padding([7, 7]).width(Length::Fill)).height(300).direction(
            scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4)
            )
        )
    ]
    .spacing(8)
    .into()
}
