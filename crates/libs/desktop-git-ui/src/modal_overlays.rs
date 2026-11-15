//! 渲染应用视图中的模态窗口。
//! 本模块只描述模态内容和交互消息，不直接承担持久化策略。

use iced::Element;



/// 模块内可见函数，执行 with_git_diff_comment 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[allow(dead_code)]
pub fn with_git_diff_comment<'a, Message: 'a>(
    editor: Option<Element<'a, Message>>,
    mut root_content: Element<'a, Message>,
) -> Element<'a, Message> {
    if let Some(editor) = editor {
        use iced::Length;
        use iced::widget::{container, opaque, stack};

        let modal_layer: Element<'_, Message> = opaque(
            container(editor)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .align_y(iced::alignment::Vertical::Top)
                .padding(iced::Padding { top: 84.0, right: 24.0, bottom: 24.0, left: 24.0 }),
        );

        root_content = stack![root_content, modal_layer].into();
    }

    root_content
}

/// 模块内可见函数，执行 with_git_diff_overlays 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn with_git_diff_overlays<'a, Message: 'a>(
    editor: Option<Element<'a, Message>>,
    discard: Option<Element<'a, Message>>,
    root_content: Element<'a, Message>,
) -> Element<'a, Message> {
    use iced::Length;
    use iced::widget::{Space, container, opaque, stack};

    let comment: Element<'a, Message> = editor
        .map(|editor| {
            opaque(
                container(editor)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .align_y(iced::alignment::Vertical::Top)
                    .padding(iced::Padding { top: 84.0, right: 24.0, bottom: 24.0, left: 24.0 }),
            )
        })
        .unwrap_or_else(|| container(Space::new()).into());

    let discard: Element<'a, Message> = discard
        .unwrap_or_else(|| container(Space::new()).into());

    let overlays: Element<'a, Message> =
        stack![comment, discard].width(Length::Fill).height(Length::Fill).into();

    stack![root_content, overlays].into()
}
