//! SHIJU 私有技能与统一技能市场命令。

use std::io::{Read, Write};

use anyhow::{Context, Result};
use clap::Subcommand;
use serde::Deserialize;
use serde_json::json;
use vw_gateway_client::{
    ShijuCredential, ShijuMaintainedSkillQuery, ShijuSkillSearchQuery,
    ShijuSystemSkillQuery, SkillMarketSkillLocatorDto, SkillMarketSkillQuery,
};

use super::context::ApiContext;
use super::input::read_json_input;
use super::output;

#[derive(Subcommand, Debug)]
pub(super) enum SkillMarketCommands {
    ShijuSearch {
        #[arg(long, value_parser = required)]
        input: String,
    },
    ShijuInstalled {
        #[arg(long, value_parser = required)]
        input: String,
    },
    ShijuSystem {
        #[arg(long, value_parser = required)]
        input: String,
    },
    ShijuMaintained {
        #[arg(long, value_parser = required)]
        input: String,
    },
    ShijuDetail {
        skill_id: u64,
        #[arg(long, value_parser = required)]
        input: String,
    },
    ShijuVersions {
        skill_id: u64,
        #[arg(long, value_parser = required)]
        input: String,
    },
    ShijuInstall {
        skill_id: u64,
        #[arg(long, value_parser = required)]
        input: String,
    },
    ShijuUninstall {
        skill_id: u64,
        #[arg(long, value_parser = required)]
        input: String,
    },
    Sources,
    Recommended {
        #[arg(long, value_parser = required)]
        source: String,
    },
    Skills {
        #[arg(long, value_parser = required)]
        source: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    Categories {
        #[arg(long, value_parser = required)]
        source: String,
    },
    Detail {
        #[arg(long, value_parser = required)]
        source: String,
        #[arg(long, value_parser = required)]
        namespace: String,
        #[arg(value_parser = required)]
        slug: String,
    },
    File {
        #[arg(long, value_parser = required)]
        source: String,
        #[arg(long, value_parser = required)]
        namespace: String,
        #[arg(value_parser = required)]
        slug: String,
    },
    Skillsets {
        #[arg(long, value_parser = required)]
        source: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    SkillsetDetail {
        #[arg(long, value_parser = required)]
        source: String,
        #[arg(value_parser = required)]
        slug: String,
    },
}

impl SkillMarketCommands {
    pub(super) fn dangerous_operation(&self) -> Option<String> {
        match self {
            Self::ShijuInstall { skill_id, .. } => {
                Some(format!("install SHIJU skill '{skill_id}'"))
            }
            Self::ShijuUninstall { skill_id, .. } => {
                Some(format!("uninstall SHIJU skill '{skill_id}'"))
            }
            _ => None,
        }
    }
}

pub(super) async fn run(
    command: SkillMarketCommands,
    context: &ApiContext,
    stdin: &mut impl Read,
    writer: &mut impl Write,
) -> Result<()> {
    let client = context.client();
    let response = match command {
        SkillMarketCommands::ShijuSearch { input } => {
            let request = query::<ShijuSkillSearchQuery>(&input, stdin)?;
            json!(
                client
                    .shiju_skill_search(&request.credential, &request.query)
                    .await
                    .map_err(anyhow::Error::msg)?
            )
        }
        SkillMarketCommands::ShijuInstalled { input } => {
            let credential = credential(&input, stdin)?;
            json!(client.shiju_skill_installed(&credential).await.map_err(anyhow::Error::msg)?)
        }
        SkillMarketCommands::ShijuSystem { input } => {
            let request = query::<ShijuSystemSkillQuery>(&input, stdin)?;
            json!(
                client
                    .shiju_system_skills(&request.credential, &request.query)
                    .await
                    .map_err(anyhow::Error::msg)?
            )
        }
        SkillMarketCommands::ShijuMaintained { input } => {
            let request = query::<ShijuMaintainedSkillQuery>(&input, stdin)?;
            json!(
                client
                    .shiju_maintained_skills(&request.credential, &request.query)
                    .await
                    .map_err(anyhow::Error::msg)?
            )
        }
        SkillMarketCommands::ShijuDetail { skill_id, input } => {
            let credential = credential(&input, stdin)?;
            json!(
                client
                    .shiju_skill_detail(&credential, skill_id)
                    .await
                    .map_err(anyhow::Error::msg)?
            )
        }
        SkillMarketCommands::ShijuVersions { skill_id, input } => {
            let credential = credential(&input, stdin)?;
            json!(
                client
                    .shiju_skill_versions(&credential, skill_id)
                    .await
                    .map_err(anyhow::Error::msg)?
            )
        }
        SkillMarketCommands::ShijuInstall { skill_id, input } => {
            let credential = credential(&input, stdin)?;
            client
                .shiju_skill_remote_install(&credential, skill_id)
                .await
                .map_err(anyhow::Error::msg)?;
            json!({"installed":true,"skill_id":skill_id})
        }
        SkillMarketCommands::ShijuUninstall { skill_id, input } => {
            let credential = credential(&input, stdin)?;
            client
                .shiju_skill_remote_uninstall(&credential, skill_id)
                .await
                .map_err(anyhow::Error::msg)?;
            json!({"uninstalled":true,"skill_id":skill_id})
        }
        SkillMarketCommands::Sources => {
            json!(client.skill_market_sources().await.map_err(anyhow::Error::msg)?)
        }
        SkillMarketCommands::Recommended { source } => {
            json!(client.skill_market_recommended(&source).await.map_err(anyhow::Error::msg)?)
        }
        SkillMarketCommands::Skills { source, input } => {
            let query = parse::<SkillMarketSkillQuery>(&input, stdin, "market query")?;
            json!(client.skill_market_skills(&source, &query).await.map_err(anyhow::Error::msg)?)
        }
        SkillMarketCommands::Categories { source } => {
            json!(client.skill_market_categories(&source).await.map_err(anyhow::Error::msg)?)
        }
        SkillMarketCommands::Detail { source, namespace, slug } => json!(
            client
                .skill_market_skill_detail(&locator(source, namespace, slug))
                .await
                .map_err(anyhow::Error::msg)?
        ),
        SkillMarketCommands::File { source, namespace, slug } => json!(
            client
                .skill_market_skill_file(&locator(source, namespace, slug))
                .await
                .map_err(anyhow::Error::msg)?
        ),
        SkillMarketCommands::Skillsets { source, input } => {
            let query = parse::<SkillMarketSkillQuery>(&input, stdin, "skillset query")?;
            json!(
                client
                    .skill_market_skill_sets(&source, &query)
                    .await
                    .map_err(anyhow::Error::msg)?
            )
        }
        SkillMarketCommands::SkillsetDetail { source, slug } => json!(
            client
                .skill_market_skill_set_detail(&source, &slug)
                .await
                .map_err(anyhow::Error::msg)?
        ),
    };
    output::write_json(writer, &response, context.output_format())
}

#[derive(Deserialize)]
struct CredentialInput {
    credential: ShijuCredential,
}
#[derive(Deserialize)]
struct CredentialQuery<T> {
    credential: ShijuCredential,
    query: T,
}

fn credential(source: &str, stdin: &mut impl Read) -> Result<ShijuCredential> {
    Ok(parse::<CredentialInput>(source, stdin, "credential")?.credential)
}
fn query<T: serde::de::DeserializeOwned>(
    source: &str,
    stdin: &mut impl Read,
) -> Result<CredentialQuery<T>> {
    parse(source, stdin, "SHIJU skill query")
}
fn parse<T: serde::de::DeserializeOwned>(
    source: &str,
    stdin: &mut impl Read,
    kind: &str,
) -> Result<T> {
    serde_json::from_value(read_json_input(source, stdin)?)
        .with_context(|| format!("invalid {kind} JSON input"))
}
fn locator(source: String, namespace: String, slug: String) -> SkillMarketSkillLocatorDto {
    SkillMarketSkillLocatorDto { source, namespace, slug }
}
fn required(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("value must not be empty".to_string());
    }
    Ok(value.to_string())
}
