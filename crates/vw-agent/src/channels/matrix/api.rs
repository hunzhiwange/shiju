use super::MatrixChannel;
use super::types::{RoomAliasResponse, WhoAmIResponse};
use matrix_sdk::{
    Client as MatrixSdkClient, SessionMeta, SessionTokens, authentication::matrix::MatrixSession,
    ruma::OwnedUserId,
};

impl MatrixChannel {
    /// 获取目标房间 ID
    pub(crate) async fn target_room_id(&self) -> anyhow::Result<String> {
        if self.room_id.starts_with('!') {
            return Ok(self.room_id.clone());
        }

        if let Some(cached) = self.resolved_room_id_cache.read().await.clone() {
            return Ok(cached);
        }

        let resolved = self.resolve_room_id().await?;
        *self.resolved_room_id_cache.write().await = Some(resolved.clone());
        Ok(resolved)
    }

    /// 获取当前认证用户的身份信息
    async fn get_my_identity(&self) -> anyhow::Result<WhoAmIResponse> {
        let url = format!("{}/_matrix/client/v3/account/whoami", self.homeserver);
        let resp = self
            .http_client
            .get(&url)
            .header("Authorization", self.auth_header_value())
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            let sanitized = crate::app::agent::providers::sanitize_api_error(&err);
            anyhow::bail!("Matrix whoami failed: {sanitized}");
        }

        Ok(resp.json().await?)
    }

    /// 获取当前认证用户的用户 ID
    pub(crate) async fn get_my_user_id(&self) -> anyhow::Result<String> {
        Ok(self.get_my_identity().await?.user_id)
    }

    /// 获取或初始化 matrix-sdk 客户端
    pub(crate) async fn matrix_client(&self) -> anyhow::Result<MatrixSdkClient> {
        let client = self
            .sdk_client
            .get_or_try_init(|| async {
                let identity = self.get_my_identity().await;
                let whoami = match identity {
                    Ok(whoami) => Some(whoami),
                    Err(error) => {
                        if self.session_owner_hint.is_some() && self.session_device_id_hint.is_some()
                        {
                            let safe_error = Self::sanitize_error_for_log(&error);
                            tracing::warn!(
                                "Matrix whoami failed; falling back to configured session hints for E2EE session restore: {safe_error}"
                            );
                            None
                        } else {
                            return Err(error);
                        }
                    }
                };

                let resolved_user_id = if let Some(whoami) = whoami.as_ref() {
                    if let Some(hinted) = self.session_owner_hint.as_ref() {
                        if hinted != &whoami.user_id {
                            tracing::warn!(
                                "Matrix configured user_id does not match whoami user_id; using whoami."
                            );
                        }
                    }
                    whoami.user_id.clone()
                } else {
                    self.session_owner_hint.clone().ok_or_else(|| {
                        anyhow::anyhow!(
                            "Matrix session restore requires user_id when whoami is unavailable"
                        )
                    })?
                };

                let resolved_device_id =
                    match (whoami.as_ref(), self.session_device_id_hint.as_ref()) {
                        (Some(whoami), Some(hinted)) => {
                            if let Some(whoami_device_id) = whoami.device_id.as_ref() {
                                if whoami_device_id != hinted {
                                    tracing::warn!(
                                        "Matrix configured device_id does not match whoami device_id; using whoami."
                                    );
                                }
                                whoami_device_id.clone()
                            } else {
                                hinted.clone()
                            }
                        }
                        (Some(whoami), None) => whoami.device_id.clone().ok_or_else(|| {
                            anyhow::anyhow!(
                                "Matrix whoami response did not include device_id. Set channels.matrix.device_id to enable E2EE session restore."
                            )
                        })?,
                        (None, Some(hinted)) => hinted.clone(),
                        (None, None) => {
                            return Err(anyhow::anyhow!(
                                "Matrix E2EE session restore requires device_id when whoami is unavailable"
                            ));
                        }
                    };

                let mut client_builder = MatrixSdkClient::builder().homeserver_url(&self.homeserver);

                if let Some(store_dir) = self.matrix_store_dir_for_user_id(&resolved_user_id) {
                    tokio::fs::create_dir_all(&store_dir).await.map_err(|error| {
                        anyhow::anyhow!(
                            "Matrix failed to initialize persistent store directory at '{}': {error}",
                            store_dir.display()
                        )
                    })?;
                    client_builder = client_builder.sqlite_store(&store_dir, None);
                }

                let client = client_builder.build().await?;
                let user_id: OwnedUserId = resolved_user_id.parse()?;
                let session = MatrixSession {
                    meta: SessionMeta {
                        user_id,
                        device_id: resolved_device_id.into(),
                    },
                    tokens: SessionTokens {
                        access_token: self.access_token.clone(),
                        refresh_token: None,
                    },
                };

                client.restore_session(session).await?;

                let holder = client.cross_process_store_locks_holder_name().to_string();
                if let Err(error) = client
                    .encryption()
                    .enable_cross_process_store_lock(holder)
                    .await
                {
                    let safe_error = Self::sanitize_error_for_log(&error);
                    tracing::warn!(
                        "Matrix failed to enable cross-process crypto-store lock: {safe_error}"
                    );
                }

                Ok::<MatrixSdkClient, anyhow::Error>(client)
            })
            .await?;

        Ok(client.clone())
    }

    /// 解析房间别名为房间 ID
    pub(crate) async fn resolve_room_id(&self) -> anyhow::Result<String> {
        let configured = self.room_id.trim();

        if configured.starts_with('!') {
            return Ok(configured.to_string());
        }

        if configured.starts_with('#') {
            let encoded_alias = Self::encode_path_segment(configured);
            let url =
                format!("{}/_matrix/client/v3/directory/room/{}", self.homeserver, encoded_alias);
            let resp = self
                .http_client
                .get(&url)
                .header("Authorization", self.auth_header_value())
                .send()
                .await?;

            if !resp.status().is_success() {
                let err = resp.text().await.unwrap_or_default();
                let sanitized = crate::app::agent::providers::sanitize_api_error(&err);
                anyhow::bail!(
                    "Matrix room alias resolution failed for '{configured}': {sanitized}"
                );
            }

            let resolved: RoomAliasResponse = resp.json().await?;
            return Ok(resolved.room_id);
        }

        anyhow::bail!(
            "Matrix room reference must start with '!' (room ID) or '#' (room alias), got: {configured}"
        )
    }

    /// 确保机器人已加入并可以访问指定房间
    async fn ensure_room_accessible(&self, room_id: &str) -> anyhow::Result<()> {
        let encoded_room = Self::encode_path_segment(room_id);
        let url =
            format!("{}/_matrix/client/v3/rooms/{}/joined_members", self.homeserver, encoded_room);
        let resp = self
            .http_client
            .get(&url)
            .header("Authorization", self.auth_header_value())
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            let sanitized = crate::app::agent::providers::sanitize_api_error(&err);
            anyhow::bail!("Matrix room access check failed for '{room_id}': {sanitized}");
        }

        Ok(())
    }

    /// 检查房间是否启用了加密
    async fn room_is_encrypted(&self, room_id: &str) -> anyhow::Result<bool> {
        let encoded_room = Self::encode_path_segment(room_id);
        let url = format!(
            "{}/_matrix/client/v3/rooms/{}/state/m.room.encryption",
            self.homeserver, encoded_room
        );
        let resp = self
            .http_client
            .get(&url)
            .header("Authorization", self.auth_header_value())
            .send()
            .await?;

        if resp.status().is_success() {
            return Ok(true);
        }

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(false);
        }

        let err = resp.text().await.unwrap_or_default();
        let sanitized = crate::app::agent::providers::sanitize_api_error(&err);
        anyhow::bail!("Matrix room encryption check failed for '{room_id}': {sanitized}");
    }

    /// 确保房间受支持
    pub(crate) async fn ensure_room_supported(&self, room_id: &str) -> anyhow::Result<()> {
        self.ensure_room_accessible(room_id).await?;

        if self.room_is_encrypted(room_id).await? {
            tracing::info!(
                "Matrix room {} is encrypted; E2EE decryption is enabled via matrix-sdk.",
                room_id
            );
        }

        Ok(())
    }

    /// 记录 E2EE 诊断信息
    pub(crate) async fn log_e2ee_diagnostics(&self, client: &MatrixSdkClient) {
        match client.encryption().get_own_device().await {
            Ok(Some(device)) => {
                if device.is_verified() {
                    tracing::info!("Matrix device is verified for E2EE.");
                } else {
                    tracing::warn!(
                        "Matrix device is not verified. Some clients may label bot messages as unverified until you sign/verify this device from a trusted session."
                    );
                }
            }
            Ok(None) => {
                tracing::warn!(
                    "Matrix own-device metadata is unavailable; verify/signing status cannot be determined."
                );
            }
            Err(error) => {
                let safe_error = Self::sanitize_error_for_log(&error);
                tracing::warn!("Matrix own-device verification check failed: {safe_error}");
            }
        }

        if client.encryption().backups().are_enabled().await {
            tracing::info!("Matrix room-key backup is enabled for this device.");
        } else {
            tracing::warn!(
                "Matrix room-key backup is not enabled for this device; `matrix_sdk_crypto::backups` warnings about missing backup keys may appear until recovery is configured."
            );
        }
    }
}

#[cfg(test)]
#[path = "api_tests.rs"]
mod api_tests;
