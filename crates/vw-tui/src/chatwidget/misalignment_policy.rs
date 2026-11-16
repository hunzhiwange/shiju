//! Terminal precaution UI for misalignment policy violations.
//! 本模块负责会话错位策略违规的阻断界面。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。

use super::*;

const MISALIGNMENT_POLICY_TITLE: &str = "Chat stopped as a precaution";
const MISALIGNMENT_POLICY_DESCRIPTION: &str = "We couldn’t confirm the agent was acting safely and following your instructions. To continue working, start or resume another chat.";

impl ChatWidget {
    // 判断 `has_misalignment_policy_violation` 对应的界面条件是否成立。
    pub(crate) fn has_misalignment_policy_violation(&self) -> bool {
        self.misalignment_policy_violation
    }

    // 计算 `rejects_misalignment_policy_op` 在当前界面流程中需要的局部结果。
    pub(crate) fn rejects_misalignment_policy_op(&self, op: &AppCommand) -> bool {
        self.misalignment_policy_violation
            && !matches!(
                op,
                AppCommand::Interrupt
                    | AppCommand::CleanBackgroundTerminals
                    | AppCommand::OverrideTurnContext { .. }
                    | AppCommand::ReloadUserConfig
                    | AppCommand::ListSkills { .. }
                    | AppCommand::SetThreadName { .. }
            )
    }

    pub(crate) fn on_misalignment_policy_violation(&mut self) {
        if self.misalignment_policy_violation {
            return;
        }

        self.misalignment_policy_violation = true;
        self.input_queue.clear();
        self.finalize_turn();
        self.refresh_pending_input_preview();
        self.bottom_pane.drain_pending_submission_state();
        self.bottom_pane
            .set_composer_text(String::new(), Vec::new(), Vec::new());
        self.bottom_pane.set_composer_input_enabled(
            /*enabled*/ false,
            Some(MISALIGNMENT_POLICY_TITLE.to_string()),
        );

        self.show_misalignment_policy_precaution();
    }

    pub(crate) fn show_misalignment_policy_precaution(&mut self) {
        let mut items = vec![
            SelectionItem {
                name: "New chat".to_string(),
                actions: vec![Box::new(|tx| tx.send(AppEvent::NewSession { name: None }))],
                dismiss_on_select: true,
                ..Default::default()
            },
            SelectionItem {
                name: "Resume another chat".to_string(),
                actions: vec![Box::new(|tx| tx.send(AppEvent::OpenResumePicker))],
                ..Default::default()
            },
        ];
        if self.remote_connection.is_some() {
            items.insert(
                1,
                SelectionItem {
                    name: "Agent command center".to_string(),
                    actions: vec![Box::new(|tx| tx.send(AppEvent::OpenAgentsOverview))],
                    ..Default::default()
                },
            );
        }
        self.bottom_pane.show_selection_view(SelectionViewParams {
            header: Box::new(
                Paragraph::new(vec![
                    Line::from(MISALIGNMENT_POLICY_TITLE).bold(),
                    Line::from(MISALIGNMENT_POLICY_DESCRIPTION).dim(),
                ])
                .wrap(Wrap { trim: false }),
            ),
            items,
            allow_cancel: false,
            ..Default::default()
        });
    }
}
