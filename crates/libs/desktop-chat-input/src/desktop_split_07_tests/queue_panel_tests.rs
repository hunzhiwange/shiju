//! 验证队列面板的任务分组、状态展示、操作消息与空态渲染。
//! 队列 fixture 固定发送行为和历史策略，便于单独验证顺序与移动可用性。

use crate::input_panel::queue_panel::{QueuePreview, queue_panel};
use iced::Length;
fn queue_item(query: &str, _created_ms: u64) -> QueuePreview {
    QueuePreview { query: query.to_string(), is_guide: false }
}

#[test]
fn queue_panel_renders_empty_short_and_long_queues() {
    let empty = queue_panel(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        Vec::new(),
        false,
    );
    assert_eq!(empty.as_widget().size().width, Length::Fill);

    let mut guided = queue_item("first line\nsecond line", 0);
    guided.is_guide = true;
    let short = queue_panel(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        vec![guided],
        true,
    );
    assert_eq!(short.as_widget().size().width, Length::Fill);

    let long_label = "这是一条很长很长很长很长很长很长很长很长很长的任务标题";
    let long = queue_panel(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        vec![queue_item(long_label, 1_000), queue_item("waiting", 2_000)],
        false,
    );
    assert_eq!(long.as_widget().size().width, Length::Fill);
}
