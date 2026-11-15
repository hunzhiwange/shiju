//! Fixed source catalog. Support describes this native domain, not upstream promises.
use vw_api_types::agent_apps::{AppCatalogEntry, AppId, AppRuntimeKind};

pub struct CatalogApp {
    pub metadata: AppCatalogEntry,
}

pub fn catalog() -> Vec<CatalogApp> {
    use AppId::*;
    [
        (Claude, "Claude", Some("claude")),
        (ClaudeDesktop, "Claude Desktop", None),
        (Codex, "Codex", Some("codex")),
        (Gemini, "Gemini", Some("gemini")),
        (GrokBuild, "Grok Build", Some("grok")),
        (OpenCode, "OpenCode", Some("opencode")),
        (OpenClaw, "OpenClaw", Some("openclaw")),
        (Hermes, "Hermes", Some("hermes")),
        (Pi, "Pi", Some("pi")),
        (Mcode, "MiniMax Code", Some("mcode")),
        (Kimi, "Kimi Code", Some("kimi")),
        (Dsh, "DeepSeek Harness", Some("dsh")),
        (Agy, "Antigravity", Some("agy")),
        (Omp, "Oh My Pi", Some("omp")),
    ]
    .into_iter()
    .map(|(id, name, cli)| CatalogApp {
        metadata: AppCatalogEntry {
            id,
            name: name.into(),
            cli_command: cli.map(str::to_owned),
            runtime_kinds: vec![if id == ClaudeDesktop {
                AppRuntimeKind::DesktopApp
            } else {
                AppRuntimeKind::NativeCli
            }],
        },
    })
    .collect()
}

/// Operations owned by application management; none is an execution permit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Discover,
    Version,
    Install,
    Update,
    Uninstall,
    Launch,
    Preferences,
}
impl Operation {
    pub const ALL: [Self; 7] = [
        Self::Discover,
        Self::Version,
        Self::Install,
        Self::Update,
        Self::Uninstall,
        Self::Launch,
        Self::Preferences,
    ];
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportState {
    Supported,
    Unsupported,
    Unverified,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capability {
    pub state: SupportState,
    pub reason: &'static str,
}
impl CatalogApp {
    pub fn capability(&self, operation: Operation) -> Capability {
        if self.metadata.id == AppId::ClaudeDesktop
            && matches!(
                operation,
                Operation::Install | Operation::Update | Operation::Uninstall | Operation::Launch
            )
        {
            return Capability {
                state: SupportState::Unsupported,
                reason: "Claude Desktop is outside the 13 CLI lifecycle/terminal allowlist; desktop discovery is separate (task 8).",
            };
        }
        let reason = match operation {
            Operation::Discover => "Discovery has not been migrated or verified (tasks 5–9).",
            Operation::Version => {
                "Version diagnostics have not been migrated or verified (tasks 10–12)."
            }
            Operation::Install => {
                "Installation recipes have not been migrated or verified (tasks 13–17)."
            }
            Operation::Update => {
                "Source-anchored updates have not been migrated or verified (task 19)."
            }
            Operation::Uninstall => {
                "Safe uninstall has not been migrated or verified (tasks 20–26)."
            }
            Operation::Launch => {
                "Authorized terminal launch has not been migrated or verified (tasks 28–32)."
            }
            Operation::Preferences => {
                "Application preferences have not been migrated or verified (task 27)."
            }
        };
        Capability { state: SupportState::Unverified, reason }
    }
}

/// References only: routing/proxy, MCP and skills remain with their mother domains.
/// Catalog registration grants no capabilities in those domains.
pub const RELATED_DOMAINS: [&str; 3] = ["mother-8:proxy", "mother-11:mcp", "mother-12:skills"];
