use super::QQChannel;
use serde_json::{Value, json};

/// 确保使用 HTTPS 协议。
pub(super) fn ensure_https(url: &str) -> anyhow::Result<()> {
    if !url.starts_with("https://") {
        anyhow::bail!(
            "Refusing to transmit sensitive data over non-HTTPS URL: URL scheme must be https"
        );
    }
    Ok(())
}

impl QQChannel {
    /// 从 QQ OAuth2 端点获取访问令牌。
    pub(super) async fn fetch_access_token(&self) -> anyhow::Result<(String, u64)> {
        let body = json!({
            "appId": self.app_id,
            "clientSecret": self.app_secret,
        });

        let resp = self.http_client().post(self.auth_url()).json(&body).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err = resp.text().await.unwrap_or_default();
            let sanitized = crate::app::agent::providers::sanitize_api_error(&err);
            anyhow::bail!("QQ token request failed ({status}): {sanitized}");
        }

        let data: Value = resp.json().await?;
        let token = data
            .get("access_token")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("Missing access_token in QQ response"))?
            .to_string();

        let expires_in = data
            .get("expires_in")
            .and_then(Value::as_str)
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(7200);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let expiry = now + expires_in.saturating_sub(60);
        Ok((token, expiry))
    }

    /// 获取有效的访问令牌。
    pub(super) async fn get_token(&self) -> anyhow::Result<String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        {
            let cache = self.token_cache.read().await;
            if let Some((ref token, expiry)) = *cache {
                if now < expiry {
                    return Ok(token.clone());
                }
            }
        }

        let (token, expiry) = self.fetch_access_token().await?;
        {
            let mut cache = self.token_cache.write().await;
            *cache = Some((token.clone(), expiry));
        }
        Ok(token)
    }

    /// 获取 WebSocket 网关 URL。
    pub(super) async fn get_gateway_url(&self, token: &str) -> anyhow::Result<String> {
        let resp = self
            .http_client()
            .get(format!("{}/gateway", self.api_base()))
            .header("Authorization", format!("QQBot {token}"))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err = resp.text().await.unwrap_or_default();
            let sanitized = crate::app::agent::providers::sanitize_api_error(&err);
            anyhow::bail!("QQ gateway request failed ({status}): {sanitized}");
        }

        let data: Value = resp.json().await?;
        let url = data
            .get("url")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("Missing gateway URL in QQ response"))?
            .to_string();

        Ok(url)
    }
}

#[cfg(test)]
#[path = "auth_tests.rs"]
mod auth_tests;
