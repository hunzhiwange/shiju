//! 分叉菜单的业务锚点与局部弹窗状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkSessionAnchor {
    pub session_id: String,
    pub directory: String,
    pub user_message_id: String,
    pub open_generation: u64,
}

pub struct ForkSessionDialog {
    pub anchor: ForkSessionAnchor,
    pub prompt: String,
    pub in_progress: bool,
}

pub fn close_fork_dialog(dialog: &mut Option<ForkSessionDialog>) {
    if !dialog.as_ref().is_some_and(|dialog| dialog.in_progress) {
        *dialog = None;
    }
}

pub fn toggle_reset_menu(dialog: &mut Option<ForkSessionDialog>, reset_index: &mut Option<usize>, message_index: usize) {
    if dialog.as_ref().is_some_and(|dialog| dialog.in_progress) {
        return;
    }
    *dialog = None;
    *reset_index = if *reset_index == Some(message_index) { None } else { Some(message_index) };
}

pub async fn fork_session(
    client: &vw_gateway_client::GatewayClient,
    anchor: &ForkSessionAnchor,
    target: super::events::ForkSessionTarget,
) -> Result<session_info::Info, String> {
    use vw_api_session::{GatewaySessionForkBody, GatewaySessionForkTarget};
    let destination = match target {
        super::events::ForkSessionTarget::Local => GatewaySessionForkTarget::SameDirectory,
        super::events::ForkSessionTarget::NewWorktree => {
            GatewaySessionForkTarget::Worktree { name: None }
        }
    };
    gateway_session::GatewaySessionClient::new(client.transport().clone())
        .session_fork::<session_info::Info>(
            &anchor.session_id,
            Some(&anchor.directory),
            &Some(GatewaySessionForkBody {
                before_turn_id: Some(anchor.user_message_id.clone()),
                target: Some(destination),
                ..Default::default()
            }),
        )
        .await
}
