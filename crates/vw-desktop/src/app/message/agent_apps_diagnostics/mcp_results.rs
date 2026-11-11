//! Dynamic diagnostic text uses the default font in both themes.
use super::mcp_state::{McpState, Phase};
use iced::{
    Element,
    widget::{column, text},
};
pub(crate) fn view<'a, Message: 'a>(state: &'a McpState) -> Element<'a, Message> {
    let mut body = column![].spacing(6);
    for (record, phase) in &state.targets {
        let phase = if state.cancelling
            && !state.terminal
            && matches!(phase, Phase::Queued | Phase::Handshake)
        {
            Phase::Cancelling
        } else {
            *phase
        };
        body = body.push(text(format!("{} · {}", record.id.0, phase.label())).size(13));
    }
    body.into()
}
