//! 插件身份构造、解析与分段校验。

use anyhow::{Context, Result, bail};

use crate::PluginIdentity;

impl PluginIdentity {
    /// 校验两个独立分段，创建冻结基线中的市场插件身份。
    pub fn marketplace(name: &str, marketplace: &str) -> Result<Self> {
        validate_segment(name, "plugin name", true)?;
        validate_segment(marketplace, "marketplace name", false)?;
        Ok(Self { name: name.to_owned(), marketplace: Some(marketplace.to_owned()) })
    }

    /// 解析精确的 `<plugin>@<marketplace>`，不添加名称别名。
    pub fn parse_marketplace(key: &str) -> Result<Self> {
        let (name, marketplace) =
            key.rsplit_once('@').context("invalid plugin key: expected <plugin>@<marketplace>")?;
        Self::marketplace(name, marketplace)
    }

    /// 返回用于发现、配置和生命周期操作的稳定标识。
    pub fn as_key(&self) -> String {
        match &self.marketplace {
            Some(marketplace) => format!("{}@{marketplace}", self.name),
            None => self.name.clone(),
        }
    }

    /// 返回原包中的插件名称。
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 返回市场归属；本地插件没有推断出的市场。
    pub fn marketplace_name(&self) -> Option<&str> {
        self.marketplace.as_deref()
    }

    pub fn local(name: &str) -> Self {
        Self { name: name.to_owned(), marketplace: None }
    }
}

fn validate_segment(value: &str, kind: &str, allow_dots: bool) -> Result<()> {
    if value.is_empty() {
        bail!("invalid {kind}: must not be empty");
    }
    if allow_dots && (value.starts_with('.') || value.ends_with('.') || value.contains("..")) {
        bail!("invalid {kind}: dots must separate non-empty name segments");
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') || allow_dots && ch == '.')
    {
        bail!("invalid {kind}: unsupported characters");
    }
    Ok(())
}
