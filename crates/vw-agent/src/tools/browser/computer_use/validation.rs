use super::super::helpers::{
    endpoint_reachable, extract_host, host_matches_allowlist, is_private_host,
};
use super::ComputerUseClient;
use serde_json::Value;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Duration;

impl ComputerUseClient {
    /// 验证并解析计算机使用服务的端点 URL
    pub(crate) fn endpoint_url(&self) -> anyhow::Result<reqwest::Url> {
        if self.config.timeout_ms == 0 {
            anyhow::bail!("browser.computer_use.timeout_ms must be > 0");
        }

        let endpoint = self.config.endpoint.trim();
        if endpoint.is_empty() {
            anyhow::bail!("browser.computer_use.endpoint cannot be empty");
        }

        let parsed = reqwest::Url::parse(endpoint).map_err(|_| {
            anyhow::anyhow!(
                "Invalid browser.computer_use.endpoint: '{endpoint}'. Expected http(s) URL"
            )
        })?;

        let scheme = parsed.scheme();
        if scheme != "http" && scheme != "https" {
            anyhow::bail!("browser.computer_use.endpoint must use http:// or https://");
        }

        let host = parsed
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("browser.computer_use.endpoint must include host"))?;

        let host_is_private = is_private_host(host);
        if !self.config.allow_remote_endpoint && !host_is_private {
            anyhow::bail!(
                "browser.computer_use.endpoint host '{host}' is public. Set browser.computer_use.allow_remote_endpoint=true to allow it"
            );
        }

        if self.config.allow_remote_endpoint && !host_is_private && scheme != "https" {
            anyhow::bail!(
                "browser.computer_use.endpoint must use https:// when allow_remote_endpoint=true and host is public"
            );
        }

        Ok(parsed)
    }

    /// 检查计算机使用服务是否可用
    pub(crate) fn available(&self) -> anyhow::Result<bool> {
        let endpoint = self.endpoint_url()?;
        Ok(endpoint_reachable(&endpoint, Duration::from_millis(500)))
    }

    /// 验证浏览器导航 URL 的安全性
    pub(crate) fn validate_url(&self, url: &str) -> anyhow::Result<()> {
        let url = url.trim();

        if url.is_empty() {
            anyhow::bail!("URL cannot be empty");
        }

        if url.starts_with("file://") {
            anyhow::bail!("file:// URLs are not allowed in browser automation");
        }

        if !url.starts_with("https://") && !url.starts_with("http://") {
            anyhow::bail!("Only http:// and https:// URLs are allowed");
        }

        if self.allowed_domains.is_empty() {
            anyhow::bail!(
                "Browser tool enabled but no allowed_domains configured. \
                Add browser.allowed_domains in system settings"
            );
        }

        let host = extract_host(url)?;

        if is_private_host(&host) {
            anyhow::bail!("Blocked local/private host: {host}");
        }

        if !host_matches_allowlist(&host, &self.allowed_domains) {
            anyhow::bail!("Host '{host}' not in browser.allowed_domains");
        }

        Ok(())
    }

    /// 验证鼠标坐标是否在允许范围内
    pub(crate) fn validate_coordinate(
        &self,
        key: &str,
        value: i64,
        max: Option<i64>,
    ) -> anyhow::Result<()> {
        if value < 0 {
            anyhow::bail!("'{key}' must be >= 0")
        }
        if let Some(limit) = max {
            if limit < 0 {
                anyhow::bail!("Configured coordinate limit for '{key}' must be >= 0")
            }
            if value > limit {
                anyhow::bail!("'{key}'={value} exceeds configured limit {limit}")
            }
        }
        Ok(())
    }

    /// 验证文件输出路径的安全性和有效性
    pub(crate) fn validate_output_path(&self, key: &str, path: &str) -> anyhow::Result<()> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            anyhow::bail!("'{key}' path cannot be empty");
        }
        if trimmed.contains('\0') {
            anyhow::bail!("'{key}' path contains invalid null byte");
        }
        let raw_path = Path::new(trimmed);
        if raw_path.is_absolute()
            || raw_path
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            anyhow::bail!("'{key}' path must stay within the workspace");
        }
        if !self.security.is_path_allowed(trimmed) {
            anyhow::bail!("'{key}' path blocked by security policy: {trimmed}");
        }
        Ok(())
    }

    /// 解析并验证文件写入路径
    pub(crate) async fn resolve_output_path_for_write(
        &self,
        key: &str,
        path: &str,
    ) -> anyhow::Result<PathBuf> {
        let trimmed = path.trim();
        self.validate_output_path(key, trimmed)?;

        tokio::fs::create_dir_all(&self.security.workspace_dir).await?;
        let workspace_root = tokio::fs::canonicalize(&self.security.workspace_dir)
            .await
            .unwrap_or_else(|_| self.security.workspace_dir.clone());

        let raw_path = Path::new(trimmed);
        let output_path = if raw_path.is_absolute() {
            raw_path.to_path_buf()
        } else {
            workspace_root.join(raw_path)
        };

        let parent = output_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("'{key}' path has no parent directory"))?;
        tokio::fs::create_dir_all(parent).await?;
        let resolved_parent = tokio::fs::canonicalize(parent).await?;
        if !self.security.is_resolved_path_allowed(&resolved_parent) {
            anyhow::bail!("{}", self.security.resolved_path_violation_message(&resolved_parent));
        }

        match tokio::fs::symlink_metadata(&output_path).await {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    anyhow::bail!(
                        "Refusing to write browser output through symlink: {}",
                        output_path.display()
                    );
                }
                if !meta.is_file() {
                    anyhow::bail!(
                        "Browser output path is not a regular file: {}",
                        output_path.display()
                    );
                }
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }

        Ok(output_path)
    }

    /// 验证浏览器自动化动作的参数
    pub(crate) fn validate_action(
        &self,
        action: &str,
        params: &serde_json::Map<String, Value>,
    ) -> anyhow::Result<()> {
        match action {
            "open" => {
                let url = params
                    .get("url")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow::anyhow!("Missing 'url' for open action"))?;
                self.validate_url(url)?;
            }
            "mouse_move" | "mouse_click" => {
                let x = read_required_i64(params, "x")?;
                let y = read_required_i64(params, "y")?;
                self.validate_coordinate("x", x, self.config.max_coordinate_x)?;
                self.validate_coordinate("y", y, self.config.max_coordinate_y)?;
            }
            "mouse_drag" => {
                let from_x = read_required_i64(params, "from_x")?;
                let from_y = read_required_i64(params, "from_y")?;
                let to_x = read_required_i64(params, "to_x")?;
                let to_y = read_required_i64(params, "to_y")?;
                self.validate_coordinate("from_x", from_x, self.config.max_coordinate_x)?;
                self.validate_coordinate("to_x", to_x, self.config.max_coordinate_x)?;
                self.validate_coordinate("from_y", from_y, self.config.max_coordinate_y)?;
                self.validate_coordinate("to_y", to_y, self.config.max_coordinate_y)?;
            }
            "key_type" => {
                let text = params
                    .get("text")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow::anyhow!("Missing 'text' for key_type action"))?;
                if text.trim().is_empty() {
                    anyhow::bail!("'text' for key_type must not be empty");
                }
                if text.len() > 4096 {
                    anyhow::bail!("'text' for key_type exceeds maximum length (4096 chars)");
                }
            }
            "key_press" => {
                let key = params
                    .get("key")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow::anyhow!("Missing 'key' for key_press action"))?;
                let valid = !key.trim().is_empty()
                    && key.len() <= 32
                    && key
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '+'));
                if !valid {
                    anyhow::bail!("'key' for key_press must be 1-32 chars of [A-Za-z0-9_+-]");
                }
            }
            "screen_capture" => {
                if let Some(path) = params.get("path").and_then(Value::as_str) {
                    self.validate_output_path("path", path)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

fn read_required_i64(params: &serde_json::Map<String, Value>, key: &str) -> anyhow::Result<i64> {
    params
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow::anyhow!("Missing or invalid '{key}' parameter"))
}
#[cfg(test)]
#[path = "validation_tests.rs"]
mod validation_tests;
