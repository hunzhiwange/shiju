//! Shortcut policy shared by host dispatch and local media lifecycle.
use super::{voice::State, voice_draft::Phase};
use desktop_voice::hotkeys::Action;
pub enum Poll {
    Skip,
    Cancel,
    Actions { actions: Vec<Action>, previous_error: Option<String> },
}
pub fn poll(state: &mut State, shutdown: bool, load: fn() -> Result<desktop_voice::preferences::Preferences, String>) -> Poll {
    if shutdown { return Poll::Skip; }
    if state.settings.busy || state.settings.edit.is_some() {
        let active = state.hotkeys.machine.active;
        state.hotkeys.reload();
        return if active { Poll::Cancel } else { Poll::Skip };
    }
    state.insertion.poll();
    let previous_error = state.hotkeys.error.clone();
    let actions = state.hotkeys.poll(|| load().map(|p| p.hotkeys));
    Poll::Actions { actions, previous_error }
}
pub fn accept_start(state: &mut State) -> Result<bool, &'static str> {
    if matches!(state.draft.phase, Phase::Recording | Phase::Transcribing) || state.chat_active() {
        state.hotkeys.machine.reset();
        // Reject the rest of this poll, including the release of the rejected press.
        return Ok(false);
    }
    if state.insertion.pending.is_some() {
        state.hotkeys.machine.reset();
        return Err("请先复制或丢弃上次未插入的听写结果。");
    }
    Ok(true)
}
pub fn finish_poll(state: &mut State) {
    if state.hotkeys.machine.active && state.draft.phase != Phase::Recording { state.hotkeys.machine.reset(); }
}
