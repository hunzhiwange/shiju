//! Read-only endpoint candidates for one authorized provider.
use crate::{
    models::preset_catalog::{claude_preset, codex_preset, gemini_grok_preset},
    providers::{endpoints, validation},
};
use serde_json::Value;
use toml_edit::{DocumentMut, Item};
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_diagnostics::DiagnosticEndpointSource::{self, Custom, Draft, Preset, SavedConfig},
};

#[derive(Debug, PartialEq, Eq)]
pub struct EndpointCandidate {
    pub url: String,
    pub sources: Vec<DiagnosticEndpointSource>,
}

/// The caller resolves the authorized provider, its pending form URLs and the
/// selected preset in the same scope. Candidates confer no network access.
/// Before probing, the host must authorize the exact target and use its shared
/// transport with redirects disabled.
///
/// First occurrence wins ordering: draft, saved config, preset, address book.
pub fn collect(
    app: AppId,
    draft_urls: &[String],
    stored: &Value,
    preset_name: Option<&str>,
) -> Result<Vec<EndpointCandidate>, String> {
    let custom = endpoints::read(stored)?;
    let mut candidates = Vec::new();
    let mut add_base = |raw: &str, source| {
        let url = raw.trim();
        if url.is_empty() {
            return Ok(());
        }
        validate(url)?;
        add(&mut candidates, url.trim_end_matches('/'), source);
        Ok::<_, String>(())
    };

    for url in draft_urls {
        add_base(url, Draft)?;
    }
    if let Some(url) = stored.get("baseUrl").and_then(Value::as_str) {
        add_base(url, SavedConfig)?;
    }
    let settings = &stored["settingsConfig"];
    match app {
        AppId::Claude | AppId::Gemini => {
            for path in ["/env/ANTHROPIC_BASE_URL", "/env/GOOGLE_GEMINI_BASE_URL"] {
                if let Some(url) = settings.pointer(path).and_then(Value::as_str) {
                    add_base(url, SavedConfig)?;
                }
            }
        }
        AppId::Codex => {
            if let Some(config) = settings.get("config").and_then(Value::as_str) {
                let doc: DocumentMut =
                    config.parse().map_err(|_| "invalid stored Codex configuration")?;
                if let Some(url) = codex_base_url(&doc) {
                    add_base(url, SavedConfig)?;
                }
            }
        }
        _ => return Err("application does not support endpoint diagnostics".into()),
    }
    if let Some(name) = preset_name.filter(|name| *name != "custom") {
        let (base, alternatives) = preset_endpoints(app, name)?;
        for url in base.into_iter().chain(alternatives.iter().map(String::as_str)) {
            // A deployment template requires values from the authorized provider form.
            if !url.contains("${") {
                add_base(url, Preset)?;
            }
        }
    }
    for endpoint in custom {
        // The address book already normalizes base URLs and preserves full request paths.
        validate(&endpoint.url)?;
        add(&mut candidates, &endpoint.url, Custom);
    }
    Ok(candidates)
}

fn validate(url: &str) -> Result<(), String> {
    validation::endpoint(url)?;
    let authority = url.split_once("://").map(|(_, rest)| rest.split('/').next().unwrap_or(""));
    // The URL parser drops empty userinfo and accepts an extra slash before the host.
    if authority.is_none_or(|value| value.is_empty() || value.contains('@')) {
        return Err("invalid application provider endpoint".into());
    }
    Ok(())
}

fn add(candidates: &mut Vec<EndpointCandidate>, url: &str, source: DiagnosticEndpointSource) {
    if let Some(existing) = candidates.iter_mut().find(|candidate| candidate.url == url) {
        if !existing.sources.contains(&source) {
            existing.sources.push(source);
        }
    } else {
        candidates.push(EndpointCandidate { url: url.into(), sources: vec![source] });
    }
}

fn preset_endpoints(
    app: AppId,
    name: &str,
) -> Result<(Option<&'static str>, &'static [String]), String> {
    let endpoints = match app {
        AppId::Claude => claude_preset(app, name).map(|preset| preset.provider()).map(|provider| {
            (provider.base_url.as_deref(), provider.endpoint_candidates.as_slice())
        }),
        AppId::Gemini => gemini_grok_preset(app, name)
            .map(|preset| (preset.base_url.as_deref(), preset.endpoint_candidates.as_slice())),
        AppId::Codex => codex_preset(app, name)
            .map(|preset| (preset.base_url.as_deref(), preset.endpoint_candidates.as_slice())),
        _ => None,
    };
    endpoints.ok_or_else(|| "unknown diagnostic endpoint preset".into())
}

fn codex_base_url(doc: &DocumentMut) -> Option<&str> {
    let selector = doc.get("model_provider").and_then(Item::as_str).map(str::trim);
    let selected = selector
        .and_then(|id| doc.get("model_providers")?.get(id)?.get("base_url")?.as_str())
        .filter(|url| !url.trim().is_empty());
    let legacy = match selector {
        None | Some("" | "openai") => doc.get("openai_base_url").and_then(Item::as_str),
        _ => None,
    };
    selected
        .or(legacy.filter(|url| !url.trim().is_empty()))
        .or_else(|| doc.get("base_url").and_then(Item::as_str))
}
