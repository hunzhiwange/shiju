//! Provider 认证、plugin hooks 与 Gateway 实例控制命令。

use std::io::{Read, Write};

use anyhow::{Result, bail};
use clap::Subcommand;
use serde_json::{Value, json};

use super::context::ApiContext;
use super::input::read_json_input;
use super::output;

/// Provider 查询与 OAuth 操作。
#[derive(Subcommand, Debug)]
pub(super) enum ProviderCommands {
    /// 列出 Gateway 可见的 provider。
    List,
    /// 列出 provider 支持的认证方式。
    Auth,
    /// 执行 provider OAuth 流程。
    Oauth {
        /// OAuth 操作。
        #[command(subcommand)]
        command: ProviderOauthCommands,
    },
}

/// Provider OAuth 操作。
#[derive(Subcommand, Debug)]
pub(super) enum ProviderOauthCommands {
    /// 启动 OAuth 授权。
    Authorize {
        /// Provider 稳定标识。
        #[arg(value_parser = parse_required_value)]
        provider_id: String,
        /// Provider 认证方式索引。
        #[arg(long)]
        method: usize,
    },
    /// 提交 OAuth 回调 code。
    Callback {
        /// Provider 稳定标识。
        #[arg(value_parser = parse_required_value)]
        provider_id: String,
        /// Provider 认证方式索引。
        #[arg(long)]
        method: usize,
        /// JSON 来源：文件、`-` stdin 或 `env:NAME`；格式为 `{"code":"..."}`。
        #[arg(long, value_parser = parse_required_value)]
        input: String,
    },
}

/// 认证、hooks 与实例控制操作。
#[derive(Subcommand, Debug)]
pub(super) enum ControlCommands {
    /// 管理 provider 认证材料。
    Auth {
        /// 认证材料操作。
        #[command(subcommand)]
        command: AuthCommands,
    },
    /// 管理 plugin hook 信任与审计。
    Hooks {
        /// Hook 操作。
        #[command(subcommand)]
        command: HookCommands,
    },
    /// 执行租户全局实例操作。
    Global {
        /// 全局实例操作。
        #[command(subcommand)]
        command: GlobalControlCommands,
    },
    /// 执行目录实例操作。
    Instance {
        /// 目录实例操作。
        #[command(subcommand)]
        command: InstanceControlCommands,
    },
}

/// Provider 认证材料操作。
#[derive(Subcommand, Debug)]
pub(super) enum AuthCommands {
    /// 从安全输入来源保存认证材料。
    Set {
        /// Provider 稳定标识。
        #[arg(value_parser = parse_required_value)]
        provider_id: String,
        /// JSON 来源：文件、`-` stdin 或 `env:NAME`；不得直接传认证材料。
        #[arg(long, value_parser = parse_required_value)]
        input: String,
    },
    /// 删除 provider 认证材料。
    Remove {
        /// Provider 稳定标识。
        #[arg(value_parser = parse_required_value)]
        provider_id: String,
    },
}

/// Plugin hook 操作。
#[derive(Subcommand, Debug)]
pub(super) enum HookCommands {
    /// 列出已发现的 hooks。
    List,
    /// 信任当前 hook hash。
    Trust {
        /// Plugin 稳定标识。
        #[arg(long, value_parser = parse_required_value)]
        plugin_id: String,
        /// 当前 hook 内容 hash。
        #[arg(long, value_parser = parse_required_value)]
        hook_hash: String,
    },
    /// 撤销当前 hook hash 的信任。
    Revoke {
        /// Plugin 稳定标识。
        #[arg(long, value_parser = parse_required_value)]
        plugin_id: String,
        /// 当前 hook 内容 hash。
        #[arg(long, value_parser = parse_required_value)]
        hook_hash: String,
    },
    /// 读取安全字段组成的 hook 命令审计。
    Audit,
}

/// 全局实例操作。
#[derive(Subcommand, Debug)]
pub(super) enum GlobalControlCommands {
    /// 释放当前租户的全部实例。
    Dispose,
}

/// 目录实例操作。
#[derive(Subcommand, Debug)]
pub(super) enum InstanceControlCommands {
    /// 释放当前目录对应的实例。
    Dispose,
}

impl ControlCommands {
    pub(super) fn dangerous_operation(
        &self,
        directory: Option<&str>,
        platform: &str,
        company: &str,
    ) -> Option<String> {
        let tenant = format!("{platform}/{company}");
        match self {
            Self::Auth { command: AuthCommands::Remove { provider_id } } => {
                Some(format!("remove auth for provider '{provider_id}' in tenant {tenant}"))
            }
            Self::Hooks { command: HookCommands::Trust { plugin_id, hook_hash } } => Some(format!(
                "trust hook '{hook_hash}' for plugin '{plugin_id}' in tenant {tenant}"
            )),
            Self::Hooks { command: HookCommands::Revoke { plugin_id, hook_hash } } => Some(
                format!("revoke hook '{hook_hash}' for plugin '{plugin_id}' in tenant {tenant}"),
            ),
            Self::Global { command: GlobalControlCommands::Dispose } => {
                Some(format!("dispose all instances for tenant {tenant}"))
            }
            Self::Instance { command: InstanceControlCommands::Dispose } => Some(format!(
                "dispose instance for directory '{}' in tenant {tenant}",
                directory.unwrap_or("<gateway-default>")
            )),
            Self::Auth { command: AuthCommands::Set { .. } }
            | Self::Hooks { command: HookCommands::List | HookCommands::Audit } => None,
        }
    }
}

pub(super) async fn run_provider<R: Read, W: Write>(
    command: ProviderCommands,
    context: &ApiContext,
    stdin: &mut R,
    writer: &mut W,
) -> Result<()> {
    let client = context.client();
    match command {
        ProviderCommands::List => {
            let response =
                client.provider_list(context.directory()).await.map_err(anyhow::Error::msg)?;
            output::write_json(writer, &response, context.output_format())
        }
        ProviderCommands::Auth => {
            let response = client.provider_auth_methods().await.map_err(anyhow::Error::msg)?;
            output::write_json(writer, &response, context.output_format())
        }
        ProviderCommands::Oauth {
            command: ProviderOauthCommands::Authorize { provider_id, method },
        } => {
            let response = client
                .provider_oauth_authorize(&provider_id, method)
                .await
                .map_err(anyhow::Error::msg)?;
            output::write_json(writer, &response, context.output_format())
        }
        ProviderCommands::Oauth {
            command: ProviderOauthCommands::Callback { provider_id, method, input },
        } => {
            let input = read_json_input(&input, stdin)?;
            let code = required_json_string(&input, "code")?;
            let response = client
                .provider_oauth_callback(&provider_id, method, code)
                .await
                .map_err(anyhow::Error::msg)?;
            output::write_json(writer, &response, context.output_format())
        }
    }
}

pub(super) async fn run<R: Read, W: Write>(
    command: ControlCommands,
    context: &ApiContext,
    stdin: &mut R,
    writer: &mut W,
) -> Result<()> {
    let client = context.client();
    let response = match command {
        ControlCommands::Auth { command: AuthCommands::Set { provider_id, input } } => {
            let info = read_json_input(&input, stdin)?;
            json!(client.auth_set(&provider_id, &info).await.map_err(anyhow::Error::msg)?)
        }
        ControlCommands::Auth { command: AuthCommands::Remove { provider_id } } => {
            json!(client.auth_remove(&provider_id).await.map_err(anyhow::Error::msg)?)
        }
        ControlCommands::Hooks { command: HookCommands::List } => {
            json!(client.hooks_get().await.map_err(anyhow::Error::msg)?)
        }
        ControlCommands::Hooks { command: HookCommands::Trust { plugin_id, hook_hash } } => {
            json!(client.hook_trust(&plugin_id, &hook_hash).await.map_err(anyhow::Error::msg)?)
        }
        ControlCommands::Hooks { command: HookCommands::Revoke { plugin_id, hook_hash } } => {
            json!(client.hook_revoke(&plugin_id, &hook_hash).await.map_err(anyhow::Error::msg)?)
        }
        ControlCommands::Hooks { command: HookCommands::Audit } => {
            json!(client.hook_audit_get().await.map_err(anyhow::Error::msg)?)
        }
        ControlCommands::Global { command: GlobalControlCommands::Dispose } => {
            let disposed = client.global_dispose().await.map_err(anyhow::Error::msg)?;
            dispose_response(context, "global", disposed)
        }
        ControlCommands::Instance { command: InstanceControlCommands::Dispose } => {
            let disposed =
                client.instance_dispose(context.directory()).await.map_err(anyhow::Error::msg)?;
            dispose_response(context, "instance", disposed)
        }
    };

    output::write_json(writer, &response, context.output_format())
}

fn dispose_response(context: &ApiContext, scope: &str, disposed: bool) -> Value {
    let tenant = context.client().endpoint().tenant_scope();
    let directory = if scope == "instance" { context.directory() } else { None };
    json!({
        "disposed": disposed,
        "target": {
            "scope": scope,
            "platform": tenant.platform_id().as_ref(),
            "company": tenant.company_id().as_ref(),
            "directory": directory,
        }
    })
}

fn required_json_string<'a>(value: &'a Value, field: &str) -> Result<&'a str> {
    let Some(value) = value.get(field).and_then(Value::as_str).map(str::trim) else {
        bail!("OAuth callback {field} must be a string");
    };
    if value.is_empty() {
        bail!("OAuth callback {field} must not be empty");
    }
    Ok(value)
}

fn parse_required_value(value: &str) -> Result<String, String> {
    if value.trim().is_empty() {
        return Err("value must not be empty".to_string());
    }
    Ok(value.to_string())
}
