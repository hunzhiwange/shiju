//! Relay protocol validation adapter for the usage recorder.
use crate::model_relay::{generation::GenerationProtocol, stream_observer::StreamObserver};

struct UsageObserver(StreamObserver);
impl agent_usage::relay::UsageStreamObserver for UsageObserver {
    fn push(&mut self, bytes: &[u8]) -> anyhow::Result<u32> {
        self.0.push(bytes).map(|update| update.text_events)
            .map_err(|_| anyhow::anyhow!("invalid usage stream"))
    }
    fn finish(&mut self) -> anyhow::Result<()> {
        self.0.finish().map(|_| ()).map_err(|_| anyhow::anyhow!("incomplete usage stream"))
    }
}

pub(crate) struct RequestUsage(agent_usage::relay::RequestUsage);
impl RequestUsage {
    pub(crate) fn new(database: agent_database::GlobalDatabase, scope: vw_api_transport::ProjectScope,
        host: String, app: String, provider: Option<String>, model: Option<String>,
        session: Option<String>, protocol: GenerationProtocol) -> anyhow::Result<Self> {
        agent_usage::relay::RequestUsage::new(database, scope, host, app, provider, model,
            session, protocol, Box::new(UsageObserver(StreamObserver::new(protocol)))).map(Self)
    }
    pub(crate) fn unknown_provider(&mut self) { self.0.unknown_provider(); }
    pub(crate) fn fail(&mut self, status: axum::http::StatusCode) { self.0.fail(status); }
    pub(crate) fn response(self, response: axum::response::Response) -> axum::response::Response {
        self.0.response(response)
    }
}
