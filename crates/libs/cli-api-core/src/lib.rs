//! 远程命令共享的连接选项、上下文与输入输出。

use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Args, ValueEnum};

pub mod context;
pub mod input;
pub mod output;

// Gateway API 命令共用的连接、认证和输出参数。
#[derive(Args, Debug)]
pub struct ApiOptions {
    /// Gateway 主机名或 IP。
    #[arg(long, global = true, default_value = "127.0.0.1")]
    host: String,

    /// Gateway 端口。
    #[arg(
        long,
        global = true,
        default_value_t = vw_config_types::gateway::DEFAULT_GATEWAY_PORT
    )]
    port: u16,

    /// 请求的平台租户 ID。
    #[arg(
        long,
        global = true,
        default_value = "local",
        value_parser = parse_required_scope_id
    )]
    platform: String,

    /// 请求的公司租户 ID。
    #[arg(
        long,
        global = true,
        default_value = "local",
        value_parser = parse_required_scope_id
    )]
    company: String,

    /// 从指定环境变量读取 Gateway skey。
    #[arg(long, global = true, conflicts_with = "skey_file")]
    skey_env: Option<String>,

    /// 从指定文件读取 Gateway skey。
    #[arg(long, global = true, conflicts_with = "skey_env")]
    skey_file: Option<PathBuf>,

    /// Gateway 请求使用的项目目录上下文。
    #[arg(long, global = true)]
    directory: Option<String>,

    /// 结构化输出格式。
    #[arg(long, global = true, value_enum, default_value_t = ApiOutputFormat::Json)]
    output: ApiOutputFormat,

    /// 确认危险或有宿主副作用的操作。
    #[arg(long, global = true)]
    yes: bool,
}

impl ApiOptions {
    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn platform(&self) -> &str {
        &self.platform
    }

    pub fn company(&self) -> &str {
        &self.company
    }

    pub fn directory(&self) -> Option<&str> {
        self.directory.as_deref()
    }

    pub fn output(&self) -> ApiOutputFormat {
        self.output
    }

    pub fn yes(&self) -> bool {
        self.yes
    }
}

/// API 结构化输出格式。
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ApiOutputFormat {
    /// 单个格式化 JSON 文档。
    Json,
    /// 每行一个紧凑 JSON 值。
    Jsonl,
}

fn parse_required_scope_id(value: &str) -> Result<String, String> {
    if value.trim().is_empty() {
        return Err("tenant scope id must not be empty".to_string());
    }
    Ok(value.to_string())
}

pub fn ensure_confirmed(yes: bool, operation: &str) -> Result<()> {
    if !yes {
        bail!("refusing dangerous operation '{operation}' without --yes");
    }
    Ok(())
}
