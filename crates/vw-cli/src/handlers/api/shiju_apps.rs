//! SHIJU 账号登录与应用管理命令。

use std::io::{Read, Write};

use anyhow::{Context, Result};
use clap::Subcommand;
use serde::Deserialize;
use serde_json::json;
use vw_gateway_client::{
    ShijuAppApiKeyBindingSaveRequest, ShijuCredential, ShijuLoginPollDto,
};

use super::context::ApiContext;
use super::input::read_json_input;
use super::output;

#[derive(Subcommand, Debug)]
pub(super) enum ShijuAppsCommands {
    Login,
    Poll { #[arg(long, value_parser = required)] input: String },
    Profile { #[arg(long, value_parser = required)] input: String },
    Catalog { #[arg(long, value_parser = required)] input: String },
    List { #[arg(long, value_parser = required)] input: String },
    ApiKeys { #[arg(long, value_parser = required)] input: String },
    BindingGet { #[arg(value_parser = required)] installation_id: String, #[arg(long, value_parser = required)] input: String },
    BindingPut { #[arg(value_parser = required)] installation_id: String, #[arg(long, value_parser = required)] input: String },
    Launch { #[arg(value_parser = required)] installation_id: String, #[arg(long, value_parser = required)] input: String },
}

impl ShijuAppsCommands {
    pub(super) fn dangerous_operation(&self) -> Option<String> {
        match self {
            Self::BindingPut { installation_id, .. } => Some(format!("update SHIJU app '{installation_id}' API key binding")),
            Self::Launch { installation_id, .. } => Some(format!("prepare SHIJU app '{installation_id}' launch")),
            Self::Login | Self::Poll { .. } | Self::Profile { .. } | Self::Catalog { .. }
            | Self::List { .. } | Self::ApiKeys { .. } | Self::BindingGet { .. } => None,
        }
    }
}

pub(super) async fn run(
    command: ShijuAppsCommands,
    context: &ApiContext,
    stdin: &mut impl Read,
    writer: &mut impl Write,
) -> Result<()> {
    let client = context.client();
    let response = match command {
        ShijuAppsCommands::Login => {
            let challenge = client.shiju_account_login().await.map_err(anyhow::Error::msg)?;
            json!({"state":challenge.state(),"authorization_url":challenge.authorization_url(),"poll_interval_seconds":challenge.poll_interval_seconds})
        }
        ShijuAppsCommands::Poll { input } => {
            let request = parse::<PollInput>(&input, stdin, "login poll")?;
            match client.shiju_account_login_poll(&request.state).await.map_err(anyhow::Error::msg)? {
                ShijuLoginPollDto::Pending => json!({"status":"pending"}),
                ShijuLoginPollDto::Approved { .. } => json!({"status":"approved"}),
            }
        }
        ShijuAppsCommands::Profile { input } => {
            let credential = credential(&input, stdin)?;
            json!(client.shiju_account_profile(&credential).await.map_err(anyhow::Error::msg)?)
        }
        ShijuAppsCommands::Catalog { input } => {
            let credential = credential(&input, stdin)?;
            json!(client.shiju_app_catalog(&credential).await.map_err(anyhow::Error::msg)?)
        }
        ShijuAppsCommands::List { input } => {
            let credential = credential(&input, stdin)?;
            let apps = client.shiju_apps(&credential).await.map_err(anyhow::Error::msg)?;
            json!({"apps":apps.apps.into_iter().map(|app| json!({
                "installation_id":app.installation_id,"app_id":app.app_id,"name":app.name,
                "description":app.description,"version":app.version,"source":app.source,
                "reconcile_status":app.reconcile_status,"runtime_status":app.runtime_status,
                "created_at":app.created_at,"updated_at":app.updated_at
            })).collect::<Vec<_>>()})
        }
        ShijuAppsCommands::ApiKeys { input } => {
            let credential = credential(&input, stdin)?;
            json!(client.shiju_app_api_keys(&credential).await.map_err(anyhow::Error::msg)?)
        }
        ShijuAppsCommands::BindingGet { installation_id, input } => {
            let credential = credential(&input, stdin)?;
            json!(client.shiju_app_api_key_binding(&credential, &installation_id).await.map_err(anyhow::Error::msg)?)
        }
        ShijuAppsCommands::BindingPut { installation_id, input } => {
            let input = parse::<BindingInput>(&input, stdin, "API key binding")?;
            client.shiju_app_save_api_key_binding(&input.credential, &installation_id, &ShijuAppApiKeyBindingSaveRequest { api_key_id: input.api_key_id }).await.map_err(anyhow::Error::msg)?;
            json!({"saved":true})
        }
        ShijuAppsCommands::Launch { installation_id, input } => {
            let credential = credential(&input, stdin)?;
            let launch = client.shiju_app_prepare_launch(&credential, &installation_id).await.map_err(anyhow::Error::msg)?;
            json!({"installation_id":launch.installation_id,"name":launch.name,"prepared":true})
        }
    };
    output::write_json(writer, &response, context.output_format())
}

#[derive(Deserialize)]
struct CredentialInput { credential: ShijuCredential }
#[derive(Deserialize)]
struct PollInput { state: String }
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BindingInput { credential: ShijuCredential, api_key_id: u64 }

fn credential(source: &str, stdin: &mut impl Read) -> Result<ShijuCredential> {
    Ok(parse::<CredentialInput>(source, stdin, "credential")?.credential)
}
fn parse<T: serde::de::DeserializeOwned>(source: &str, stdin: &mut impl Read, kind: &str) -> Result<T> {
    serde_json::from_value(read_json_input(source, stdin)?)
        .with_context(|| format!("invalid SHIJU {kind} JSON input"))
}
fn required(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() { return Err("value must not be empty".to_string()); }
    Ok(value.to_string())
}
