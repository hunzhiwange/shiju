//! 输入面板内容布局；跨领域子视图由桌面装配。
use iced::{Element, Length};
use iced::widget::{column, Space};

pub struct Content<'a, Message> {
    pub attachment_strip: Element<'a, Message>,
    pub file_refs: Element<'a, Message>,
    pub task_mode_form: Element<'a, Message>,
    pub input: Element<'a, Message>,
    pub divider: Element<'a, Message>,
    pub bottom_bar: Element<'a, Message>,
    pub context_card: Option<Element<'a, Message>>,
    pub todo_panel: Option<Element<'a, Message>>,
    pub queue_panel: Option<Element<'a, Message>>,
}

pub fn content<'a, Message: 'a>(parts: Content<'a, Message>) -> Element<'a, Message> {
    let Content { attachment_strip, file_refs, task_mode_form, input, divider, bottom_bar, context_card, todo_panel, queue_panel } = parts;
    if let Some(todo_panel) = todo_panel {
        // 有待办事项时，将 TODO 面板紧贴输入框上方
        let mut input_block = column![todo_panel].spacing(0);
        if let Some(context_card) = context_card {
            input_block = input_block.push(context_card);
        }
        input_block = input_block.push(input).push(divider).push(bottom_bar);

        column![attachment_strip, file_refs, task_mode_form, input_block].spacing(3)
    } else if let Some(queue_panel) = queue_panel {
        // 队列布局：队列面板 + 标准内容
        let mut queued = column![
            queue_panel,
            Space::new().height(Length::Fixed(4.0)),
            attachment_strip,
            file_refs,
            task_mode_form
        ]
        .spacing(6);
        if let Some(context_card) = context_card {
            queued = queued.push(context_card);
        }
        queued.push(input).push(divider).push(bottom_bar)
    } else {
        // 标准布局：文件引用 + 任务模式表单 + 输入框 + 底部栏
        let mut standard = column![attachment_strip, file_refs, task_mode_form].spacing(6);
        if let Some(context_card) = context_card {
            standard = standard.push(context_card);
        }
        standard.push(input).push(divider).push(bottom_bar)
    }.into()

}
