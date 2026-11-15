//! Ordered quick-message editing and persistence outcomes.
use config_ui::{QuickMessageConfig, QuickMessageGroup, QuickMessageKind};
use super::system_settings_themes::update::ToastKind;
#[derive(Debug, Clone)]
pub enum QuickMessagesMessage {
    Add, Remove(usize), TextChanged(usize, String), GroupChanged(usize, QuickMessageGroup),
    KindChanged(usize, QuickMessageKind), MoveUp(usize), MoveDown(usize), Save, Saved(Result<(), String>),
}
pub enum Effect {
    None,
    Save(Vec<QuickMessageConfig>),
    Toast(ToastKind, &'static str, Vec<desktop_i18n::format::Argument>),
}
pub fn update(items: &mut Vec<QuickMessageConfig>, message: QuickMessagesMessage) -> Effect {
    match message {
        QuickMessagesMessage::Add => {
            items.push(QuickMessageConfig {
                text: "新快捷消息".to_string(),
                group: QuickMessageGroup::General,
                kind: QuickMessageKind::FillInput,
            });
            Effect::None
        }
        QuickMessagesMessage::Remove(index) => {
            if index < items.len() {
                items.remove(index);
            }
            Effect::None
        }
        QuickMessagesMessage::TextChanged(index, text) => {
            if let Some(item) = items.get_mut(index) {
                item.text = text;
            }
            Effect::None
        }
        QuickMessagesMessage::GroupChanged(index, group) => {
            if let Some(item) = items.get_mut(index) {
                item.group = group;
            }
            Effect::None
        }
        QuickMessagesMessage::KindChanged(index, kind) => {
            if let Some(item) = items.get_mut(index) {
                item.kind = kind;
            }
            Effect::None
        }
        QuickMessagesMessage::MoveUp(index) => {
            if index > 0 && index < items.len() {
                items.swap(index, index - 1);
            }
            Effect::None
        }
        QuickMessagesMessage::MoveDown(index) => {
            if let Some(next_index) = index.checked_add(1)
                && next_index < items.len()
            {
                items.swap(index, next_index);
            }
            Effect::None
        }
        QuickMessagesMessage::Save => {
            if items.iter().any(|item| item.text.trim().is_empty()) {
                return Effect::Toast(ToastKind::Warning, "快捷消息内容不能为空", vec![]);
            }

            Effect::Save(items.clone())
        }

        QuickMessagesMessage::Saved(Ok(())) => Effect::Toast(
            ToastKind::Success,
            "快捷消息已保存",
            vec![],
        ),
        QuickMessagesMessage::Saved(Err(error)) => Effect::Toast(
            ToastKind::Error,
            "保存快捷消息失败：%s",
            vec![error.into()],
        ),
    }
}

#[cfg(test)]
#[path = "quick_messages_update_tests.rs"]
mod tests;
