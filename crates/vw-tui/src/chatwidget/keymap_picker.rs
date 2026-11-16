//! `ChatWidget` integration points for the `/keymap` picker flow.
//!
//! The picker model, capture view, and edit semantics live in [`crate::keymap_setup`]. This module
//! keeps only the `ChatWidget`-owned responsibilities: opening those views in the bottom pane,
//! routing users back to the right picker row after an edit, and synchronizing the committed
//! keymap config back into the live widget state. Keeping these methods outside `chatwidget.rs`
//! keeps the main transcript/event surface from also owning the `/keymap` navigation details.
//!
//! The important invariant is that any accepted keymap edit must update three places together:
//! the stored `Config.tui_keymap`, the cached copy-response binding used by app-level shortcuts,
//! and the bottom pane's runtime keymap bindings. Updating only one of those would make the UI
//! appear to accept a remap while some handlers still respond to the old keys.
//! 本模块负责`/keymap` 选择器与实时键位配置同步。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。
//! 接受编辑后必须同时更新持久化配置、应用级复制快捷键缓存和底部面板运行时绑定。
//! 返回选择器时优先替换旧子菜单；视图栈已变化时重新打开选择器，避免落到陈旧界面。

use codex_config::types::TuiKeymap;
use codex_terminal_detection::terminal_info;

use super::ChatWidget;
use super::queued_message_edit_hint_binding;
use crate::app_event::KeymapCaptureMode;
use crate::app_event::KeymapEditIntent;
use crate::keymap::RuntimeKeymap;
use crate::keymap_setup;

impl ChatWidget {
    /// Opens the root `/keymap` picker using the current `tui.keymap` configuration.
    ///
    /// This validates the persisted keymap before building picker rows because every subsequent
    /// picker screen needs the effective runtime bindings, including preset defaults and user
    /// overrides. If the config is invalid, the user sees the parse error instead of a partial
    /// picker that could commit edits against stale runtime state.
    /// 使用当前的“tui.keymap”配置打开根“/keymap”选择器。这会在生成选择器行之前验证保留的键映射，因为每个后续选择器屏幕都需要有效的运行时绑定，包括预设默认值和用户覆盖。如果配置无效，用户将看到解析错误，
    /// 而不是可以根据过时的运行时状态提交编辑的部分选择器。
    pub(crate) fn open_keymap_picker(&mut self) {
        match RuntimeKeymap::from_config(&self.config.tui_keymap) {
            Ok(runtime_keymap) => {
                let params = keymap_setup::build_keymap_picker_params_with_filter(
                    &runtime_keymap,
                    &self.config.tui_keymap,
                    self.keymap_action_filter(),
                );
                self.bottom_pane.show_selection_view(params);
            }
            Err(err) => {
                self.add_error_message(format!("Invalid `tui.keymap` configuration: {err}"));
            }
        }
    }

    /// Opens the per-action menu for one keymap action.
    ///
    /// Callers pass the already-resolved runtime keymap from the app event that selected the
    /// action. Recomputing it here would risk showing a menu for a different config if another
    /// keymap edit was applied between the picker event and this handler.
    /// 打开某个键位操作的编辑菜单。调用方传入选择该操作时已解析的运行时键位映射；若在选择事件与此处理器之间又发生编辑，此处重新计算可能会错误展示另一份配置。
    pub(crate) fn open_keymap_action_menu(
        &mut self,
        context: String,
        action: String,
        runtime_keymap: &RuntimeKeymap,
    ) {
        let params = keymap_setup::build_keymap_action_menu_params(
            context,
            action,
            runtime_keymap,
            &self.config.tui_keymap,
        );
        self.bottom_pane.show_selection_view(params);
    }

    /// Opens the key-capture view for a set, replace, or alternate-binding edit.
    ///
    /// The capture view owns raw key interpretation, but `ChatWidget` supplies the event sender so
    /// the captured key can come back through the same app-event path as menu selections. Bypassing
    /// that path would skip config persistence and leave the runtime keymap cache unchanged.
    /// 打开新增、替换或备用绑定编辑所需的按键捕获视图。捕获视图负责解释原始按键，`ChatWidget` 提供事件发送方，使捕获结果沿菜单选择所用的同一应用事件路径返回；绕过该路径会跳过配置持久化，
    /// 并保持运行时键盘映射缓存不变。
    pub(crate) fn open_keymap_capture(
        &mut self,
        context: String,
        action: String,
        intent: KeymapEditIntent,
        capture_mode: KeymapCaptureMode,
        runtime_keymap: &RuntimeKeymap,
    ) {
        let view = keymap_setup::build_keymap_capture_view(
            context,
            action,
            intent,
            capture_mode,
            runtime_keymap,
            self.app_event_tx.clone(),
        );
        self.bottom_pane.show_view(Box::new(view));
        self.request_redraw();
    }

    /// Opens the keypress inspector with the current runtime bindings.
    /// 使用当前运行时绑定打开按键检查器。
    pub(crate) fn open_keymap_debug(&mut self, runtime_keymap: &RuntimeKeymap) {
        let view = keymap_setup::build_keymap_debug_view(runtime_keymap, &self.config.tui_keymap);
        self.bottom_pane.show_view(Box::new(view));
        self.request_redraw();
    }

    /// Opens the menu that lets the user choose which existing binding to replace.
    ///
    /// This is only used for actions with multiple effective bindings. The chosen binding is
    /// carried through the subsequent capture intent so replacement edits do not accidentally
    /// collapse alternate bindings that should remain available.
    /// 打开允许用户选择要替换的现有绑定的菜单。这仅用于具有多个有效绑定的操作。所选绑定通过后续捕获意图进行，因此替换编辑不会意外折叠应保持可用的替代绑定。
    pub(crate) fn open_keymap_replace_binding_menu(
        &mut self,
        context: String,
        action: String,
        runtime_keymap: &RuntimeKeymap,
    ) {
        let params =
            keymap_setup::build_keymap_replace_binding_menu_params(context, action, runtime_keymap);
        self.bottom_pane.show_selection_view(params);
    }

    /// Returns to the root picker with the edited action selected.
    ///
    /// The preferred path replaces any active keymap picker submenu in place so the bottom-pane
    /// back stack does not accumulate obsolete menus after each edit. If the expected view stack is
    /// no longer active, this falls back to showing a fresh picker rather than dropping the user on
    /// a stale screen.
    /// 返回根选择器并选中刚编辑的操作。优先替换当前键位映射子菜单，避免底部面板返回栈累积过期菜单；若预期视图栈已失效，则重新打开选择器，避免停留在陈旧界面。
    pub(crate) fn return_to_keymap_picker(
        &mut self,
        context: &str,
        action: &str,
        runtime_keymap: &RuntimeKeymap,
    ) {
        let params = keymap_setup::build_keymap_picker_params_for_selected_action_with_filter(
            runtime_keymap,
            &self.config.tui_keymap,
            self.keymap_action_filter(),
            context,
            action,
        );
        let replaced = self.bottom_pane.replace_active_views_with_selection_view(
            &[
                keymap_setup::KEYMAP_PICKER_VIEW_ID,
                keymap_setup::KEYMAP_ACTION_MENU_VIEW_ID,
                keymap_setup::KEYMAP_REPLACE_BINDING_MENU_VIEW_ID,
            ],
            params,
        );
        if !replaced {
            let params = keymap_setup::build_keymap_picker_params_for_selected_action_with_filter(
                runtime_keymap,
                &self.config.tui_keymap,
                self.keymap_action_filter(),
                context,
                action,
            );
            self.bottom_pane.show_selection_view(params);
        }
        self.request_redraw();
    }

    fn keymap_action_filter(&self) -> keymap_setup::KeymapActionFilter {
        keymap_setup::KeymapActionFilter {
            fast_mode_enabled: self.fast_mode_enabled(),
        }
    }

    /// Applies a committed keymap edit to the live chat widget.
    ///
    /// The caller is responsible for persisting the config file before invoking this method. This
    /// method updates the in-memory config, app-level copy binding cache, and bottom-pane keymap
    /// bindings as one unit; callers that update only `self.config.tui_keymap` would leave visible
    /// picker state and active key handlers disagreeing until the next restart.
    /// 将已提交的键位编辑应用到当前聊天组件。调用方须先持久化配置文件；本方法会一并更新内存配置、应用级复制快捷键缓存和底部面板运行时绑定。
    /// 若只更新 `self.config.tui_keymap`，可见选择器状态与实际按键处理器会一直不一致，直至下次启动。
    pub(crate) fn apply_keymap_update(
        &mut self,
        keymap_config: TuiKeymap,
        runtime_keymap: &RuntimeKeymap,
    ) {
        self.config.tui_keymap = keymap_config;
        self.copy_last_response_binding = runtime_keymap.app.copy.clone();
        self.chat_keymap = runtime_keymap.chat.clone();
        self.queued_message_edit_hint_binding =
            queued_message_edit_hint_binding(runtime_keymap, terminal_info());
        self.bottom_pane
            .set_queued_message_edit_binding(self.queued_message_edit_hint_binding);
        self.bottom_pane.set_keymap_bindings(runtime_keymap);
        self.request_redraw();
    }
}
