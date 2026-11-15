use vw_api_external_session::*;
use super::reader;
use std::path::PathBuf;
use gateway_external_session::AgentAppsReadError;
#[derive(Clone, Copy, Debug)]
pub enum Destination { Clipboard, File }
#[derive(Clone, Debug)]
pub enum Event {
    Start(Destination, bool),
    Loaded(reader::Scope, Destination, Result<SessionMarkdownResponse, AgentAppsReadError>),
    Selected(reader::Scope, String, Option<PathBuf>),
    Saved(reader::Scope, bool),
}
