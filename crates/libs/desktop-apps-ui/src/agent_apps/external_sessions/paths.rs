use vw_api_external_session::*;
use super::Scope;
#[derive(Clone, Copy, Debug)]
pub enum Location { Source, Project }

#[derive(Clone, Debug)]
pub enum Event {
    Sources(bool),
    Copy(ExternalSessionIdentity, u64, Location),
    Reveal(ExternalSessionIdentity, u64, Location),
    Finished(Scope, Result<(), String>),
}
