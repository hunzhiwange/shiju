//! Shared inventory of resolved runtime bindings.
//! 已解析运行时快捷键的共享清单。
//!
//! `/keymap` lookup and chord-prefix conflict detection must inspect the same
//! actions. Declaring those actions once prevents a new binding from appearing
//! in the picker while being silently omitted from chord validation.
//! `/keymap` 查询与组合键前缀冲突检测必须检查同一批动作；集中声明可防止新绑定只出现在
//! 选择器中，却被组合键校验静默遗漏。

use super::RuntimeKeymap;
use crate::key_hint::KeyBinding;
use codex_config::types::KeybindingsSpec;
use codex_config::types::TuiKeymap;
use std::sync::Arc;

/// Config context in which a keymap action is active.
/// 快捷键动作生效时所处的配置上下文。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum KeymapContext {
    Global,
    Chat,
    Composer,
    Editor,
    VimNormal,
    VimOperator,
    VimTextObject,
    Pager,
    List,
    Agents,
    Approval,
}

impl KeymapContext {
    pub(crate) const fn config_name(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Chat => "chat",
            Self::Composer => "composer",
            Self::Editor => "editor",
            Self::VimNormal => "vim_normal",
            Self::VimOperator => "vim_operator",
            Self::VimTextObject => "vim_text_object",
            Self::Pager => "pager",
            Self::List => "list",
            Self::Agents => "agents",
            Self::Approval => "approval",
        }
    }

    pub(crate) const fn allows_plain_chord_prefix(self) -> bool {
        matches!(
            self,
            Self::VimNormal | Self::VimOperator | Self::VimTextObject
        )
    }

    pub(crate) fn overlaps(self, other: Self) -> bool {
        // 共享主界面与编辑器上下文会沿同一输入路径竞争，因此必须参与交叉冲突检测。
        if self == other {
            return true;
        }

        matches!(
            (self, other),
            (Self::List, Self::Approval)
                | (Self::Approval, Self::List)
                | (Self::List, Self::Agents)
                | (Self::Agents, Self::List)
                | (Self::Chat, Self::List)
                | (Self::List, Self::Chat)
        ) || self.is_shared_main() && other.is_main_editor()
            || other.is_shared_main() && self.is_main_editor()
            || self.is_shared_main() && other.is_shared_main()
    }

    const fn is_shared_main(self) -> bool {
        matches!(self, Self::Global | Self::Chat | Self::Composer)
    }

    const fn is_main_editor(self) -> bool {
        matches!(
            self,
            Self::Editor | Self::VimNormal | Self::VimOperator | Self::VimTextObject
        )
    }
}

/// Stable identity for one configurable action.
/// 一个可配置动作的稳定标识，由上下文和配置动作名共同组成。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct KeymapActionId {
    pub(crate) context: KeymapContext,
    pub(crate) action: &'static str,
}

impl KeymapActionId {
    pub(crate) fn config_path(self) -> String {
        format!("tui.keymap.{}.{}", self.context.config_name(), self.action)
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct RuntimeActionBinding<'a> {
    // 只借用运行时解析结果，供编辑器和冲突检查读取，不复制或改写绑定。
    pub(super) id: KeymapActionId,
    pub(super) bindings: &'a [KeyBinding],
}

macro_rules! runtime_group_mut {
    ($runtime_keymap:expr, editor) => {
        Arc::make_mut(&mut $runtime_keymap.editor)
    };
    ($runtime_keymap:expr, $group:ident) => {
        &mut $runtime_keymap.$group
    };
}

macro_rules! define_runtime_action_bindings {
    ($($context:literal => $context_id:ident, $group:ident, $config_group:ident [$($action:ident),+ $(,)?]),+ $(,)?) => {
        /// Resolve a config context/action pair to its runtime identity.
        /// 将配置中的上下文与动作二元组解析为运行时稳定标识。
        pub(crate) fn keymap_action_id(
            context: &str,
            action: &str,
        ) -> Option<KeymapActionId> {
            match (context, action) {
                $(
                    $(
                        ($context, stringify!($action)) => Some(KeymapActionId {
                            context: KeymapContext::$context_id,
                            action: stringify!($action),
                        }),
                    )+
                )+
                _ => None,
            }
        }

        /// Return the root-config slot for one runtime action.
        /// 返回某个运行时动作在根配置中的槽位。
        pub(super) fn configured_binding_for_action(
            keymap: &TuiKeymap,
            action: KeymapActionId,
        ) -> Option<&Option<KeybindingsSpec>> {
            match (action.context.config_name(), action.action) {
                $(
                    $(
                        ($context, stringify!($action)) => {
                            Some(&keymap.$config_group.$action)
                        }
                    )+
                )+
                _ => None,
            }
        }

        /// Iterate over every stable runtime action identity.
        /// 按声明顺序遍历全部稳定运行时动作标识。
        pub(crate) fn keymap_action_ids() -> impl Iterator<Item = KeymapActionId> {
            [
                $(
                    $(
                        KeymapActionId {
                            context: KeymapContext::$context_id,
                            action: stringify!($action),
                        },
                    )+
                )+
            ]
            .into_iter()
        }

        /// Return resolved bindings for one stable config context and action.
        /// 返回指定稳定配置上下文与动作已解析出的用户绑定。
        pub(crate) fn bindings_for_action<'a>(
            runtime_keymap: &'a RuntimeKeymap,
            context: &str,
            action: &str,
        ) -> Option<&'a [KeyBinding]> {
            match (context, action) {
                $(
                    $(
                        ($context, stringify!($action)) => {
                            Some(super::user_bindings(
                                runtime_keymap.$group.$action.as_slice(),
                            ))
                        }
                    )+
                )+
                _ => None,
            }
        }

        /// Append an internal dispatch binding to one resolved action.
        /// 给已解析动作追加内部派发绑定，不改变其面向用户的配置标识。
        pub(super) fn push_binding_for_action(
            runtime_keymap: &mut RuntimeKeymap,
            action: KeymapActionId,
            binding: KeyBinding,
        ) -> bool {
            match (action.context.config_name(), action.action) {
                $(
                    $(
                        ($context, stringify!($action)) => {
                            runtime_group_mut!(runtime_keymap, $group).$action.push(binding);
                            true
                        }
                    )+
                )+
                _ => false,
            }
        }

        /// Iterate over the same runtime bindings exposed to the keymap editor.
        /// 遍历快捷键编辑器所展示的同一份运行时绑定清单。
        pub(super) fn runtime_action_bindings(
            runtime_keymap: &RuntimeKeymap,
        ) -> impl Iterator<Item = RuntimeActionBinding<'_>> {
            [
                $(
                    $(
                        RuntimeActionBinding {
                            id: KeymapActionId {
                                context: KeymapContext::$context_id,
                                action: stringify!($action),
                            },
                            bindings: super::user_bindings(
                                runtime_keymap.$group.$action.as_slice(),
                            ),
                        },
                    )+
                )+
            ]
            .into_iter()
        }
    };
}

define_runtime_action_bindings! {
    // 该声明同时生成查询、配置访问和内部派发映射，新增动作必须只在这里接线一次。
    "global" => Global, app, global [
        open_agents,
        open_transcript,
        open_external_editor,
        copy,
        clear_terminal,
        toggle_vim_mode,
        toggle_fast_mode,
        toggle_raw_output,
        toggle_side_conversation,
    ],
    "chat" => Chat, chat, chat [
        interrupt_turn,
        decrease_reasoning_effort,
        increase_reasoning_effort,
        previous_permission_mode,
        next_permission_mode,
        edit_queued_message,
    ],
    "composer" => Composer, composer, composer [
        submit,
        queue,
        toggle_shortcuts,
        history_search_previous,
        history_search_next,
    ],
    "editor" => Editor, editor, editor [
        insert_newline,
        move_left,
        move_right,
        move_up,
        move_down,
        move_word_left,
        move_word_right,
        move_line_start,
        move_line_end,
        delete_backward,
        delete_forward,
        delete_backward_word,
        delete_forward_word,
        kill_line_start,
        kill_whole_line,
        kill_line_end,
        yank,
    ],
    "vim_normal" => VimNormal, vim_normal, vim_normal [
        enter_insert,
        append_after_cursor,
        append_line_end,
        insert_line_start,
        open_line_below,
        open_line_above,
        move_left,
        move_right,
        move_up,
        move_down,
        move_word_forward,
        move_word_backward,
        move_word_end,
        move_line_start,
        move_line_end,
        delete_char,
        replace_char,
        substitute_char,
        delete_to_line_end,
        change_to_line_end,
        yank_line,
        paste_after,
        start_delete_operator,
        start_yank_operator,
        start_change_operator,
        cancel_operator,
    ],
    "vim_operator" => VimOperator, vim_operator, vim_operator [
        delete_line,
        yank_line,
        motion_left,
        motion_right,
        motion_up,
        motion_down,
        motion_word_forward,
        motion_word_backward,
        motion_word_end,
        motion_line_start,
        motion_line_end,
        select_inner_text_object,
        select_around_text_object,
        cancel,
    ],
    "vim_text_object" => VimTextObject, vim_text_object, vim_text_object [
        word,
        big_word,
        parentheses,
        brackets,
        braces,
        double_quote,
        single_quote,
        backtick,
        cancel,
    ],
    "pager" => Pager, pager, pager [
        scroll_up,
        scroll_down,
        page_up,
        page_down,
        half_page_up,
        half_page_down,
        jump_top,
        jump_bottom,
        close,
        close_transcript,
    ],
    "list" => List, list, list [
        move_up,
        move_down,
        move_left,
        move_right,
        page_up,
        page_down,
        jump_top,
        jump_bottom,
        accept,
        cancel,
    ],
    "agents" => Agents, agents, agents [
        search,
        new_task,
        rename,
        stop,
        toggle_grouping,
    ],
    "approval" => Approval, approval, approval [
        open_fullscreen,
        open_thread,
        approve,
        approve_for_session,
        approve_for_prefix,
        deny,
        decline,
        cancel,
    ],
}
