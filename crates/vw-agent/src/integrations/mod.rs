//! 集成目录模型，描述可用状态、分类及配置驱动的激活结果。

pub mod registry;

use crate::app::agent::config::Config;
use anyhow::Result;

/// Integration status
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum IntegrationStatus {
    /// Fully implemented and ready to use
    Available,
    /// Configured and active
    Active,
    /// Planned but not yet implemented
    ComingSoon,
}

/// Integration category
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum IntegrationCategory {
    Chat,
    AiModel,
    Productivity,
    MusicAudio,
    SmartHome,
    ToolsAutomation,
    MediaCreative,
    Social,
    Platform,
}

impl IntegrationCategory {
    pub fn label(self) -> &'static str {
        match self {
            Self::Chat => "Chat Providers",
            Self::AiModel => "AI Models",
            Self::Productivity => "Productivity",
            Self::MusicAudio => "Music & Audio",
            Self::SmartHome => "Smart Home",
            Self::ToolsAutomation => "Tools & Automation",
            Self::MediaCreative => "Media & Creative",
            Self::Social => "Social",
            Self::Platform => "Platforms",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Chat,
            Self::AiModel,
            Self::Productivity,
            Self::MusicAudio,
            Self::SmartHome,
            Self::ToolsAutomation,
            Self::MediaCreative,
            Self::Social,
            Self::Platform,
        ]
    }
}

/// A registered integration
pub struct IntegrationEntry {
    pub name: &'static str,
    pub description: &'static str,
    pub category: IntegrationCategory,
    pub status_fn: fn(&Config) -> IntegrationStatus,
}

/// Handle the `integrations` CLI command
pub fn handle_command(command: IntegrationCommands, config: &Config) -> Result<()> {
    match command {
        IntegrationCommands::List { category, status } => {
            list_integrations(config, category.as_deref(), status.as_deref())
        }
        IntegrationCommands::Search { query } => search_integrations(config, &query),
        IntegrationCommands::Info { name } => show_integration_info(config, &name),
    }
}

fn status_icon(status: IntegrationStatus) -> &'static str {
    match status {
        IntegrationStatus::Active => "✅",
        IntegrationStatus::Available => "⚪",
        IntegrationStatus::ComingSoon => "🔜",
    }
}

fn parse_category_filter(input: &str) -> Option<IntegrationCategory> {
    match input.to_lowercase().as_str() {
        "chat" => Some(IntegrationCategory::Chat),
        "ai" | "model" | "models" | "ai-model" | "ai-models" => Some(IntegrationCategory::AiModel),
        "productivity" => Some(IntegrationCategory::Productivity),
        "music" | "audio" | "music-audio" => Some(IntegrationCategory::MusicAudio),
        "smart-home" | "smarthome" | "home" => Some(IntegrationCategory::SmartHome),
        "tools" | "automation" | "tools-automation" => Some(IntegrationCategory::ToolsAutomation),
        "media" | "creative" | "media-creative" => Some(IntegrationCategory::MediaCreative),
        "social" => Some(IntegrationCategory::Social),
        "platform" | "platforms" => Some(IntegrationCategory::Platform),
        _ => None,
    }
}

fn parse_status_filter(input: &str) -> Option<IntegrationStatus> {
    match input.to_lowercase().as_str() {
        "active" => Some(IntegrationStatus::Active),
        "available" => Some(IntegrationStatus::Available),
        "coming-soon" | "comingsoon" | "soon" => Some(IntegrationStatus::ComingSoon),
        _ => None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn list_integrations(
    config: &Config,
    category_filter: Option<&str>,
    status_filter: Option<&str>,
) -> Result<()> {
    let entries = registry::all_integrations();

    let cat_filter = category_filter.map(parse_category_filter);
    if let Some(None) = cat_filter.as_ref() {
        anyhow::bail!(
            "Unknown category: '{}'. Valid: chat, ai, productivity, music, smart-home, tools, media, social, platform",
            category_filter.unwrap_or_default()
        );
    }
    let cat_filter = cat_filter.flatten();

    let stat_filter = status_filter.map(parse_status_filter);
    if let Some(None) = stat_filter.as_ref() {
        anyhow::bail!(
            "Unknown status: '{}'. Valid: active, available, coming-soon",
            status_filter.unwrap_or_default()
        );
    }
    let stat_filter = stat_filter.flatten();

    let mut count = 0usize;
    for cat in IntegrationCategory::all() {
        if let Some(ref cf) = cat_filter {
            if *cf != *cat {
                continue;
            }
        }

        let cat_entries: Vec<_> = entries
            .iter()
            .filter(|e| e.category == *cat)
            .filter(|e| {
                if let Some(ref sf) = stat_filter { (e.status_fn)(config) == *sf } else { true }
            })
            .collect();

        if cat_entries.is_empty() {
            continue;
        }

        println!();
        println!("  {}", console::style(cat.label()).bold().underlined());
        for entry in &cat_entries {
            let status = (entry.status_fn)(config);
            println!(
                "    {} {:<20} {}",
                status_icon(status),
                entry.name,
                console::style(entry.description).dim()
            );
            count += 1;
        }
    }

    println!();
    println!("  {} integration(s) shown.", count);
    println!();
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn list_integrations(
    _config: &Config,
    _category_filter: Option<&str>,
    _status_filter: Option<&str>,
) -> Result<()> {
    anyhow::bail!("Listing integrations is not supported on WASM")
}

#[cfg(not(target_arch = "wasm32"))]
fn search_integrations(config: &Config, query: &str) -> Result<()> {
    let entries = registry::all_integrations();
    let query_lower = query.to_lowercase();

    let matches: Vec<_> = entries
        .iter()
        .filter(|e| {
            e.name.to_lowercase().contains(&query_lower)
                || e.description.to_lowercase().contains(&query_lower)
        })
        .collect();

    if matches.is_empty() {
        println!();
        println!("  No integrations matching '{query}'.");
        println!();
        return Ok(());
    }

    println!();
    for entry in &matches {
        let status = (entry.status_fn)(config);
        println!(
            "    {} {:<20} {} — {}",
            status_icon(status),
            entry.name,
            console::style(entry.category.label()).dim(),
            entry.description,
        );
    }
    println!();
    println!("  {} result(s) for '{query}'.", matches.len());
    println!();
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn search_integrations(_config: &Config, _query: &str) -> Result<()> {
    anyhow::bail!("Searching integrations is not supported on WASM")
}

#[cfg(not(target_arch = "wasm32"))]
fn show_integration_info(config: &Config, name: &str) -> Result<()> {
    let entries = registry::all_integrations();
    let name_lower = name.to_lowercase();

    let Some(entry) = entries.iter().find(|e| e.name.to_lowercase() == name_lower) else {
        anyhow::bail!("Unknown integration: {name}. Check README for supported integrations.");
    };

    let status = (entry.status_fn)(config);
    let icon = status_icon(status);
    let label = match status {
        IntegrationStatus::Active => "Active",
        IntegrationStatus::Available => "Available",
        IntegrationStatus::ComingSoon => "Coming Soon",
    };

    println!();
    println!("  {} {} — {}", icon, console::style(entry.name).white().bold(), entry.description);
    println!("  Category: {}", entry.category.label());
    println!("  Status:   {label}");
    println!();

    // Show setup hints based on integration
    match entry.name {
        "Telegram" => {
            println!("  Setup:");
            println!("    1. Message @BotFather on Telegram");
            println!("    2. Create a bot and copy the token");
            println!("    3. Start: vibewindow channel start");
        }
        "Discord" => {
            println!("  Setup:");
            println!("    1. Go to https://discord.com/developers/applications");
            println!("    2. Create app → Bot → Copy token");
            println!("    3. Enable MESSAGE CONTENT intent");
        }
        "Slack" => {
            println!("  Setup:");
            println!("    1. Go to https://api.slack.com/apps");
            println!("    2. Create app → Bot Token Scopes → Install");
        }
        "OpenRouter" => {
            println!("  Setup:");
            println!("    1. Get API key at https://openrouter.ai/keys");
            println!("    Access 200+ models with one key.");
        }
        "Ollama" => {
            println!("  Setup:");
            println!("    1. Install: brew install ollama");
            println!("    2. Pull a model: ollama pull llama3");
            println!("    3. Set provider to 'ollama' in system settings");
        }
        "iMessage" => {
            println!("  Setup (macOS only):");
            println!("    Uses AppleScript bridge to send/receive iMessages.");
            println!("    Requires Full Disk Access in System Settings → Privacy.");
        }
        "GitHub" => {
            println!("  Setup:");
            println!("    1. Create a personal access token at https://github.com/settings/tokens");
            println!("    2. Add to config: [integrations.github] token = \"ghp_...\"");
        }
        "Browser" => {
            println!("  Built-in:");
            println!("    VibeWindow can control Chrome/Chromium for web tasks.");
            println!("    Uses headless browser automation.");
        }
        "Cron" => {
            println!("  Built-in:");
            println!(
                "    Schedule tasks in {}/",
                vw_config_types::paths::tilde_config_path("workspace/cron")
            );
            println!("    Run: vibewindow cron list");
        }
        "Webhooks" => {
            println!("  Built-in:");
            println!("    HTTP endpoint for external triggers.");
            println!("    Run: vibewindow gateway");
        }
        _ => {
            if status == IntegrationStatus::ComingSoon {
                println!("  This integration is planned. Stay tuned!");
                println!("  Track progress: https://github.com/theonlyhennygod/vibewindow");
            }
        }
    }

    println!();
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn show_integration_info(_config: &Config, _name: &str) -> Result<()> {
    anyhow::bail!("Showing integration info is not supported on WASM")
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use clap::Subcommand;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Subcommand)]
pub enum IntegrationCommands {
    List {
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        status: Option<String>,
    },
    Search {
        query: String,
    },
    Info {
        name: String,
    },
}
