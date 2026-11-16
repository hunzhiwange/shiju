//! Chat-widget wiring for the `/ide` command and IDE context prompt injection.
//! 本模块负责`/ide` 命令与 IDE 上下文注入。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。

use codex_app_server_protocol::UserInput;

use super::ChatWidget;

#[derive(Default)]
/// 保存 `IdeContextState` 在当前流程中的展示状态。
pub(super) struct IdeContextState {
    enabled: bool,
    prompt_fetch_warned: bool,
}

impl IdeContextState {
    // 判断 `is_enabled` 所表示的当前界面条件。
    pub(super) fn is_enabled(&self) -> bool {
        self.enabled
    }

    // 计算 `enable` 所需的界面结果。
    fn enable(&mut self) {
        self.enabled = true;
        self.prompt_fetch_warned = false;
    }

    fn disable(&mut self) {
        self.enabled = false;
        self.prompt_fetch_warned = false;
    }

    fn mark_available(&mut self) {
        self.prompt_fetch_warned = false;
    }
}

impl ChatWidget {
    // 处理 `handle_ide_command` 对应的界面事件并同步相关状态。
    pub(super) fn handle_ide_command(&mut self) {
        if self.ide_context.is_enabled() {
            self.ide_context.disable();
            self.sync_ide_context_status_indicator();
            self.add_info_message("IDE context is off.".to_string(), /*hint*/ None);
        } else {
            self.ide_context.enable();
            self.add_ide_context_status_message();
        }
    }

    pub(super) fn handle_ide_command_args(&mut self, args: &str) {
        match args.to_ascii_lowercase().as_str() {
            "" => self.handle_ide_command(),
            "on" => {
                self.ide_context.enable();
                self.add_ide_context_status_message();
            }
            "off" => {
                self.ide_context.disable();
                self.sync_ide_context_status_indicator();
                self.add_info_message("IDE context is off.".to_string(), /*hint*/ None);
            }
            "status" => {
                self.add_ide_context_status_message();
            }
            _ => {
                self.add_error_message("Usage: /ide [on|off|status]".to_string());
            }
        }
    }

    /// Fetches fresh IDE context for the outgoing user turn and folds it into the prompt.
    /// 获取传出用户TURN的新IDE上下文，并将其折叠到提示符中。
    // 计算 `maybe_apply_ide_context` 所需的界面结果。
    pub(super) fn maybe_apply_ide_context(&mut self, items: &mut Vec<UserInput>) {
        if !self.ide_context.is_enabled() {
            return;
        }

        match crate::ide_context::fetch_ide_context(&self.config.cwd, &self.config.codex_home) {
            Ok(context) => {
                self.ide_context.mark_available();
                self.sync_ide_context_status_indicator();
                crate::ide_context::apply_ide_context_to_user_input(&context, items);
            }
            Err(err) => {
                self.sync_ide_context_status_indicator();
                if !self.ide_context.prompt_fetch_warned {
                    self.ide_context.prompt_fetch_warned = true;
                    self.add_info_message(
                        "IDE context was skipped for this message.".to_string(),
                        Some(err.prompt_skip_hint()),
                    );
                }
            }
        }
    }

    fn add_ide_context_status_message(&mut self) {
        if !self.ide_context.is_enabled() {
            self.sync_ide_context_status_indicator();
            self.add_info_message("IDE context is off.".to_string(), /*hint*/ None);
            return;
        }

        match crate::ide_context::fetch_ide_context(&self.config.cwd, &self.config.codex_home) {
            Ok(context) => {
                self.ide_context.mark_available();
                self.sync_ide_context_status_indicator();
                if crate::ide_context::has_prompt_context(&context) {
                    self.add_info_message(
                        "IDE context is on.".to_string(),
                        Some(
                            "Future messages will include your current IDE selection and open tabs."
                                .to_string(),
                        ),
                    );
                } else {
                    self.add_info_message(
                        "IDE context is on.".to_string(),
                        Some("Connected to your IDE.".to_string()),
                    );
                }
            }
            Err(err) => {
                self.ide_context.disable();
                self.sync_ide_context_status_indicator();
                self.add_info_message(
                    "IDE context could not be enabled.".to_string(),
                    Some(err.user_facing_hint()),
                );
            }
        }
    }

    pub(super) fn sync_ide_context_status_indicator(&mut self) {
        self.bottom_pane
            .set_ide_context_active(self.ide_context.is_enabled());
    }
}
