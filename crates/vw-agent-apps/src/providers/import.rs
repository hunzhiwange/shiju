//! Dispatch policy is separate from application-specific native decoding.
use vw_api_types::{agent_apps::AppId, app_providers::import::ImportTrigger};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportStrategy {
    Skip,
    Default,
    Incremental,
}
pub fn strategy(
    app: AppId,
    trigger: ImportTrigger,
    has_rows: bool,
) -> Result<ImportStrategy, &'static str> {
    use AppId::*;
    if app == ClaudeDesktop {
        return Err("use compatible providers from Claude");
    }
    // Dsh merges multiple profiles and resolves credential references. A generic
    // settings snapshot cannot faithfully represent this native operation.
    if app == Dsh {
        return Err("Dsh native profile import is unsupported");
    }
    let incremental = matches!(app, OpenCode | OpenClaw | Hermes | Pi | Omp | Mcode | Kimi);
    if trigger == ImportTrigger::Startup {
        return Ok(if incremental || has_rows {
            ImportStrategy::Skip
        } else {
            ImportStrategy::Default
        });
    }
    // Manual Grok import may restore its official entry even beside custom rows.
    if app == GrokBuild {
        return Ok(ImportStrategy::Default);
    }
    Ok(if incremental {
        ImportStrategy::Incremental
    } else if has_rows {
        ImportStrategy::Skip
    } else {
        ImportStrategy::Default
    })
}
/// Placeholder credentials must never become permanent catalog credentials.
pub(super) fn reject_placeholders(
    bytes: &[u8],
) -> Result<(), super::live::write_guard::WriteError> {
    if [b"PROXY_MANAGED".as_slice(), b"CC_SWITCH_PENDING_KEY".as_slice()]
        .iter()
        .any(|marker| bytes.windows(marker.len()).any(|part| part == *marker))
    {
        return Err(super::live::write_guard::WriteError::InvalidResource);
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub enum NativeImport {
    Claude(vw_api_types::app_providers::claude::ClaudeProfile),
    Codex(vw_api_types::app_providers::codex_catalog::CodexProfile),
    Gemini(vw_api_types::app_providers::gemini::GeminiProfile),
    Grok(vw_api_types::app_providers::grok::GrokProfile),
    Agy(vw_api_types::app_providers::agy::AgyProfile),
    OpenCode(vw_api_types::app_providers::opencode::OpenCodeProfile),
    OpenClaw(vw_api_types::app_providers::openclaw::OpenClawProfile),
    Hermes(vw_api_types::app_providers::hermes::HermesProfile),
    Pi(vw_api_types::app_providers::pi::PiProfile),
    Omp(vw_api_types::app_providers::omp::OmpProfile),
    Mcode(vw_api_types::app_providers::mcode::McodeProfile),
    Kimi(vw_api_types::app_providers::kimi::KimiProfile),
}
/// All file paths are fixed or resolved by the existing bounded application reader.
/// No write guards, native locks, directories, login changes or processes are created.
#[cfg(target_os = "macos")]
pub fn read(
    home: &std::path::Path,
    app: AppId,
    trigger: ImportTrigger,
) -> Result<Vec<(String, NativeImport)>, super::live::write_guard::WriteError> {
    use super::live::write_guard::WriteError;
    let file = |parts: &[&str]| {
        super::catalog::read_file(home, parts).map_err(|_| WriteError::InvalidResource)
    };
    let required = |parts: &[&str]| file(parts)?.ok_or(WriteError::InvalidResource);
    let single = |p| vec![("default".into(), p)];
    macro_rules! declarations {
        ($owner:ident, $variant:ident, $bytes:expr) => {{
            let bytes = $bytes?;
            reject_placeholders(bytes.as_deref().unwrap_or_default())?;
            super::$owner::declarations(bytes.as_deref())?
                .into_iter()
                .map(|(id, p)| (id, NativeImport::$variant(p)))
                .collect()
        }};
    }
    Ok(match app {
        AppId::Claude => {
            let b = required(&[".claude", "settings.json"])?;
            reject_placeholders(&b)?;
            single(NativeImport::Claude(super::claude::import("default", &b)?))
        }
        AppId::Codex => single(NativeImport::Codex(super::codex_catalog::import(
            file(&[".codex", "config.toml"])?.as_deref(),
            file(&[".codex", "auth.json"])?.as_deref(),
        )?)),
        AppId::Gemini => single(NativeImport::Gemini(super::gemini::import(
            &required(&[".gemini", ".env"])?,
            file(&[".gemini", "settings.json"])?.as_deref(),
        )?)),
        AppId::GrokBuild => match super::grok::import(
            &required(&[".grok", "config.toml"])?,
            trigger == ImportTrigger::Startup,
        )? {
            Some(p) => {
                let id = if matches!(
                    p.config,
                    vw_api_types::app_providers::grok::GrokConfig::Official {}
                ) {
                    "grokbuild-official"
                } else {
                    "default"
                };
                vec![(id.into(), NativeImport::Grok(p))]
            }
            None => vec![],
        },
        AppId::Agy => {
            super::agy::import(&required(&[".gemini", "antigravity-cli", "settings.json"])?)
                .map_err(|_| WriteError::InvalidResource)?
                .map(NativeImport::Agy)
                .map(single)
                .unwrap_or_default()
        }
        AppId::OpenCode => {
            let bytes = match file(&[".config", "opencode", "opencode.jsonc"])? {
                Some(b) => Some(b),
                None => file(&[".config", "opencode", "opencode.json"])?,
            };
            declarations!(opencode, OpenCode, Ok::<_, WriteError>(bytes))
        }
        AppId::OpenClaw => declarations!(openclaw, OpenClaw, super::openclaw::read(home)),
        AppId::Hermes => declarations!(hermes, Hermes, super::hermes::read(home)),
        AppId::Pi => declarations!(pi, Pi, super::pi::read(home)),
        AppId::Omp => declarations!(omp, Omp, super::omp::read(home)),
        AppId::Mcode => declarations!(mcode, Mcode, super::mcode::read(home)),
        AppId::Kimi => declarations!(kimi, Kimi, super::kimi::read(home)),
        AppId::Dsh | AppId::ClaudeDesktop => return Err(WriteError::InvalidResource),
    })
}
