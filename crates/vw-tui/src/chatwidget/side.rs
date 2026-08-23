//! Chat widget hooks for side-conversation mode.
//!
//! App-level side-thread lifecycle lives in `app::side`; this module owns the
//! chat-surface pieces that side mode toggles, such as the composer placeholder,
//! footer label, and inline `/side` message submission behavior.
//! 本模块负责side 对话模式的 ChatWidget 展示适配。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。
//! 应用层拥有 side 生命周期；这里仅同步输入占位符、页脚标签和内置 `/side` 消息提交。

use super::*;

impl ChatWidget {
    // 计算 `submit_user_message_as_plain_user_turn` 在当前界面流程中需要的局部结果。
    pub(crate) fn submit_user_message_as_plain_user_turn(
        &mut self,
        user_message: UserMessage,
    ) -> Option<AppCommand> {
        self.submit_user_message_with_shell_escape_policy(user_message, ShellEscapePolicy::Disallow)
    }

    pub(crate) fn set_side_conversation_active(&mut self, active: bool) {
        self.active_side_conversation = active;
        let placeholder = if active {
            self.side_placeholder_text.clone()
        } else {
            self.normal_placeholder_text.clone()
        };
        self.bottom_pane.set_placeholder_text(placeholder);
        self.bottom_pane.set_side_conversation_active(active);
        if self.blocks_direct_input && !active {
            self.bottom_pane.set_parent_owned_thread();
        }
    }

    pub(crate) fn side_conversation_active(&self) -> bool {
        self.active_side_conversation
    }

    pub(crate) fn set_side_conversation_context_label(&mut self, label: Option<String>) {
        self.bottom_pane.set_side_conversation_context_label(label);
    }
}
