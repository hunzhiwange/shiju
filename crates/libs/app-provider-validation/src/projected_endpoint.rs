use vw_api_app_provider_base::ProviderProtocol;
use vw_api_app_provider_edit::universal_sync::UniversalSyncError;
use vw_api_app_identity::{AppId};

pub fn projected_endpoint(
    app: AppId,
    base_url: &str,
) -> Result<(ProviderProtocol, String), UniversalSyncError> {
    match app {
        AppId::Claude => Ok((ProviderProtocol::Anthropic, base_url.into())),
        AppId::Codex => {
            let base = base_url.trim_end_matches('/');
            let origin =
                reqwest::Url::parse(base).map_err(|_| UniversalSyncError::InvalidProfile)?.path()
                    == "/";
            let base_url = if origin { format!("{base}/v1") } else { base.into() };
            Ok((ProviderProtocol::OpenaiResponses, base_url))
        }
        AppId::Gemini => Ok((ProviderProtocol::Gemini, base_url.into())),
        _ => Err(UniversalSyncError::UnsupportedApp),
    }
}
