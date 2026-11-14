use super::super::GatewayClient;
use vw_api_types::agent_apps_skills::search_api::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarketSearchError {
    Transport,
    Decode,
    UnexpectedHttp { status: u16 },
    Http { status: u16, failure: MarketSearchFailure },
}
impl std::fmt::Display for MarketSearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport => f.write_str("market search transport failed"),
            Self::Decode => f.write_str("market search response invalid"),
            Self::UnexpectedHttp { status } => write!(f, "market search HTTP {status}"),
            Self::Http { status, failure } => write!(f, "market search HTTP {status}: {failure:?}"),
        }
    }
}
impl std::error::Error for MarketSearchError {}
impl GatewayClient {
    pub async fn agent_apps_market_search(&self, query: &MarketSearchQuery, token: &str)
        -> Result<MarketSearchResponse, MarketSearchError> {
        let mut credential = reqwest::header::HeaderValue::from_str(token)
            .map_err(|_| MarketSearchError::Transport)?;
        credential.set_sensitive(true);
        let request = self.native_transport().map_err(|_| MarketSearchError::Transport)?
            .get(format!("{}/v1/agent-apps/skills/search", self.endpoint.base_url()))
            .query(query).header("x-agent-apps-token", credential);
        let response = crate::http::apply_request_context(request, &self.endpoint)
            .send().await.map_err(|_| MarketSearchError::Transport)?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|_| MarketSearchError::Transport)?;
        if !status.is_success() {
            let failure = serde_json::from_slice(&bytes)
                .map_err(|_| MarketSearchError::UnexpectedHttp { status: status.as_u16() })?;
            return Err(MarketSearchError::Http { status: status.as_u16(), failure });
        }
        serde_json::from_slice(&bytes).map_err(|_| MarketSearchError::Decode)
    }
}
