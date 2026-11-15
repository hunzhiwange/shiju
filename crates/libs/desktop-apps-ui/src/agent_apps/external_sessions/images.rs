use vw_api_external_session::*;
use super::reader;
use iced::widget::image;
use gateway_external_session::AgentAppsReadError;
#[derive(Clone, Default)]
pub struct State {
    pub entries: Vec<Entry>,
    pub enlarged: Option<ImageRef>,
}
#[derive(Clone)]
pub struct Entry {
    pub reference: ImageRef,
    pub handle: Option<image::Handle>,
    pub serial: u64,
    pub pending: bool,
    pub abort: Option<iced::task::Handle>,
}
impl State {
    pub fn cancel(&self) {
        for entry in &self.entries {
            if let Some(abort) = &entry.abort {
                abort.abort();
            }
        }
    }
}
#[derive(Clone, Debug)]
pub enum Event {
    Load(ImageRef),
    Enlarge(ImageRef),
    Close,
    Loaded(reader::Scope, u64, ImageRef, Result<image::Handle, AgentAppsReadError>),
}
