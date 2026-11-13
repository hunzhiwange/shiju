//! Accurate help for the pinned editor; all text uses the normal UI font.
use super::{App, Message};
use crate::app::components::system_settings_common::rounded_action_btn_style;
use crate::app::message::editor::EditorMessage;
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Element, Length};

pub(super) fn entries(mac: bool, vim: bool, folding: bool) -> Vec<(&'static str, String)> {
    let command = if mac { "Cmd" } else { "Ctrl" };
    let word = if mac { "Option" } else { "Ctrl" };
    let alt = if mac { "Option" } else { "Alt" };
    let mut rows = vec![
        ("普通编辑", String::new()),
        ("全选", format!("{command}+A")),
        ("复制 / 剪切 / 粘贴优先于 Vim", format!("{command}+C / X / V")),
        ("撤销 / 重做", format!("{command}+Z / {command}+Shift+Z")),
        ("保存请求由应用处理", format!("{command}+S")),
        ("Tab 插入所选缩进：空格 2/4/8 或 Tab 字符", "Tab".into()),
        ("向后焦点导航，不是反缩进", "Shift+Tab".into()),
        ("按词移动；加 Shift 扩展选择", format!("{word}+Left / Right")),
        ("按词删除", format!("{word}+Backspace / Delete")),
        ("Home 先到首个非空白，再到行首", "Home".into()),
        ("搜索导航", String::new()),
        ("搜索", format!("{command}+F")),
        ("替换", format!("{command}+H")),
        ("下一个 / 上一个匹配", "F3 / Shift+F3".into()),
        ("搜索字段循环（反向加 Shift）", "Tab / Shift+Tab".into()),
        ("跳转到行", format!("{command}+G")),
        ("命令面板", format!("{command}+Shift+P")),
        ("关闭当前浮层或搜索", "Escape".into()),
        ("多光标与折叠", String::new()),
        ("选择下一个相同项", format!("{command}+D")),
        ("向上 / 向下添加光标", format!("Ctrl+{alt}+Up / Down")),
    ];
    if folding {
        rows.extend([("切换折叠", "Ctrl+.".into()), ("全部折叠 / 展开", "Ctrl+K / Ctrl+J".into())]);
    }
    if vim {
        rows.extend([
            ("Vim（单光标 MVP）", String::new()),
            ("移动：方向、单词、行首尾、文件首尾；支持计数", "hjkl / w b e / 0 ^ $ / gg G".into()),
            ("进入插入模式", "i a I A o O".into()),
            ("字符 / 行选择", "v / V".into()),
            ("删除 / 修改 / 复制；整行操作", "d c y / dd cc yy".into()),
            ("删除字符 / 寄存器粘贴（不等同系统剪贴板）", "x / p P".into()),
            ("撤销 / 重做", "u / Ctrl+R".into()),
            ("搜索 / 下一个 / 上一个", "/ / n N".into()),
            ("跳到第 N 行", ":N".into()),
            ("请求应用保存；保存后退出 Vim", ":w / :wq".into()),
            ("仅退出 Vim，保留文件", ":q".into()),
            ("返回 Normal", "Escape".into()),
        ]);
    }
    if !folding {
        for (label, _) in &mut rows {
            if *label == "多光标与折叠" {
                *label = "多光标";
            }
        }
    }
    if vim {
        rows.retain(|(label, _)| !matches!(*label, "选择下一个相同项" | "向上 / 向下添加光标"));
    }
    rows
}

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    let editor = app
        .preview_tabs
        .iter()
        .find(|tab| Some(&tab.path) == app.active_preview_path.as_ref())
        .map(|tab| &tab.editor.inner);
    let vim = editor.is_some_and(|editor| editor.vim_enabled());
    let folding = editor.is_some_and(|editor| editor.folding_enabled());
    let mut body = iced::widget::Column::<Message>::new().spacing(8);
    for (label, key) in entries(cfg!(target_os = "macos"), vim, folding) {
        body = body.push(if key.is_empty() {
            column![text(i18n.__(label)).size(14)]
        } else {
            column![text(i18n.__(label)).size(12), text(key).size(11)].spacing(2)
        });
    }
    column![
        row![
            text(i18n.__("编辑器使用帮助")).size(14),
            Space::new().width(Length::Fill),
            button(text(i18n.__("关闭")))
                .padding([4, 8])
                .style(rounded_action_btn_style)
                .on_press(Message::Editor(EditorMessage::ClosePopover))
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
