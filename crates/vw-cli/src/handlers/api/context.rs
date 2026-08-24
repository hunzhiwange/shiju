//! Gateway 连接、租户与认证上下文。

use std::ffi::OsString;

use anyhow::{Context, Result, bail};
use vw_gateway_client::vw_api_types::id::TenantScope;
use vw_gateway_client::{GatewayAuth, GatewayClient, GatewayEndpoint};

use super::{ApiArgs, ApiOutputFormat};

/// 一次根级 Gateway API 命令调用复用的上下文。
#[derive(Debug)]
pub(super) struct ApiContext {
    client: GatewayClient,
    directory: Option<String>,
    output_format: ApiOutputFormat,
}

impl ApiContext {
    pub(super) fn from_args(args: &ApiArgs) -> Result<Self> {
        Self::from_args_with_env(args, |name| std::env::var_os(name))
    }

    pub(super) fn from_args_with_env(
        args: &ApiArgs,
        env_lookup: impl Fn(&str) -> Option<OsString>,
    ) -> Result<Self> {
        let tenant_scope = TenantScope::new(args.platform.clone(), args.company.clone())
            .context("invalid API tenant scope")?;
        let skey = resolve_skey(args, env_lookup)?;

        let mut endpoint =
            GatewayEndpoint::new(args.host.clone(), args.port).with_tenant_scope(tenant_scope);
        if skey.is_some() {
            endpoint = endpoint.with_auth(GatewayAuth { skey });
        }
        let client = GatewayClient::new(endpoint).map_err(anyhow::Error::msg)?;

        Ok(Self { client, directory: args.directory.clone(), output_format: args.output })
    }

    pub(super) fn client(&self) -> &GatewayClient {
        &self.client
    }

    pub(super) fn directory(&self) -> Option<&str> {
        self.directory.as_deref()
    }

    pub(super) fn output_format(&self) -> ApiOutputFormat {
        self.output_format
    }
}

fn resolve_skey(
    args: &ApiArgs,
    env_lookup: impl Fn(&str) -> Option<OsString>,
) -> Result<Option<String>> {
    match (&args.skey_env, &args.skey_file) {
        (Some(_), Some(_)) => bail!("choose only one skey source"),
        (Some(variable), None) => {
            let variable = variable.trim();
            if variable.is_empty() {
                bail!("skey environment variable name is empty");
            }
            let value = env_lookup(variable)
                .with_context(|| format!("skey environment variable {variable} is not set"))?
                .into_string()
                .map_err(|_| {
                    anyhow::anyhow!("skey environment variable {variable} is not UTF-8")
                })?;
            normalized_skey(value, "skey environment variable")
        }
        (None, Some(path)) => {
            let value = std::fs::read_to_string(path)
                .with_context(|| format!("failed to read skey file {}", path.display()))?;
            normalized_skey(value, "skey file")
        }
        (None, None) => Ok(None),
    }
}

fn normalized_skey(value: String, source: &str) -> Result<Option<String>> {
    let value = value.trim();
    if value.is_empty() {
        bail!("{source} is empty");
    }
    Ok(Some(value.to_string()))
}
