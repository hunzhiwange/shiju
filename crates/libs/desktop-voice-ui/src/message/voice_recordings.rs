use desktop_voice::recordings_playback::Replay;
use voice_recordings::{self as recordings, Entry};
use gateway_endpoint::GatewayEndpoint;
#[derive(Default)]
pub struct State {
    pub entries: Vec<Entry>,
    pub result: Option<String>,
    pub problem: Option<String>,
    pub replay: Option<Replay>,
    pub busy: bool,
    pub initialized: bool,
    pub generation: u64,
    pub scope: Option<super::voice_draft::Scope>,
    pub endpoint: Option<GatewayEndpoint>,
    pub cancel: Option<tokio::sync::watch::Sender<bool>>,
    pub native_cancel: Option<desktop_voice::local::Cancellation>,
}
#[derive(Clone)]
pub enum Event {
    Refresh,
    Play(String),
    Stop,
    Delete(String),
    Clear,
    Retry(String),
    Apply,
    Dismiss,
    Tick(u64),
    Retried(u64, Result<String, String>),
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("VoiceRecordingsEvent").field(&std::mem::discriminant(self)).finish()
    }
}
impl State {
    pub fn stop(&mut self) {
        if let Some(cancel) = self.native_cancel.take() {
            cancel.cancel();
        }
        self.replay.take();
        self.cancel.take();
        self.busy = false;
        self.result = None;
        self.generation = self.generation.wrapping_add(1);
        self.scope = None;
        self.endpoint = None;
    }
    pub fn refresh(&mut self, path: Result<std::path::PathBuf, String>) -> Result<(), String> {
        self.entries = recordings::Store::open(&path?)?.list()?;
        self.problem = None;
        Ok(())
    }
}
