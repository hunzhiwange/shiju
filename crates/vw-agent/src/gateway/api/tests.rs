use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use crate::app::agent::config::schema::{
        CloudflareTunnelConfig, LarkReceiveMode, NgrokTunnelConfig, WatiConfig,
    };
    use crate::app::agent::gateway::api::integrations::{
        apply_integration_credentials_update, build_integration_settings_payload, config_revision,
    };
    use crate::app::agent::gateway::api::secrets::{
        MASKED_SECRET, hydrate_config_for_save, mask_sensitive_fields,
    };
    use std::collections::BTreeMap;

    #[test]
    fn masking_keeps_toml_valid() {
        let mut cfg = crate::app::agent::config::Config::default();
        cfg.api_key = Some("sk-live-123".to_string());

        let masked = mask_sensitive_fields(&cfg);
        let toml = toml::to_string_pretty(&masked).expect("masked config should serialize");
        let parsed: crate::app::agent::config::Config =
            toml::from_str(&toml).expect("masked config should remain valid TOML for Config");

        assert_eq!(parsed.api_key.as_deref(), Some(MASKED_SECRET));
    }

    #[test]
    fn hydrate_config_for_save_restores_masked_secrets_and_paths() {
        let mut current = crate::app::agent::config::Config::default();
        current.config_path = std::path::PathBuf::from("/tmp/current/vibewindow.json");
        current.workspace_dir = std::path::PathBuf::from("/tmp/current/workspace");
        current.api_key = Some("real-key".to_string());

        let mut incoming = mask_sensitive_fields(&current);
        incoming.default_model = Some("gpt-4.1-mini".to_string());

        let hydrated = hydrate_config_for_save(incoming, &current);

        assert_eq!(hydrated.config_path, current.config_path);
        assert_eq!(hydrated.workspace_dir, current.workspace_dir);
        assert_eq!(hydrated.api_key, current.api_key);
        assert_eq!(hydrated.default_model.as_deref(), Some("gpt-4.1-mini"));
    }

    #[test]
    fn mask_sensitive_fields_covers_wati_email_and_feishu_secrets() {
        let mut cfg = crate::app::agent::config::Config::default();
        cfg.proxy.http_proxy = Some("http://user:pass@proxy.internal:8080".to_string());
        cfg.proxy.https_proxy = Some("https://user:pass@proxy.internal:8443".to_string());
        cfg.proxy.all_proxy = Some("socks5://user:pass@proxy.internal:1080".to_string());
        cfg.tunnel.cloudflare =
            Some(CloudflareTunnelConfig { token: "cloudflare-real-token".to_string() });
        cfg.tunnel.ngrok = Some(NgrokTunnelConfig {
            auth_token: "ngrok-real-token".to_string(),
            domain: Some("vibewindow.ngrok.app".to_string()),
        });
        cfg.channels_config.wati = Some(WatiConfig {
            api_token: "wati-real-token".to_string(),
            api_url: "https://live-mt-server.wati.io".to_string(),
            tenant_id: Some("tenant-1".to_string()),
            allowed_numbers: vec!["*".to_string()],
        });
        let mut email = crate::app::agent::channels::email_channel::EmailConfig::default();
        email.password = "email-real-password".to_string();
        cfg.channels_config.email = Some(email);
        cfg.channels_config.feishu = Some(crate::app::agent::config::FeishuConfig {
            app_id: "cli_app_id".to_string(),
            app_secret: "feishu-real-secret".to_string(),
            encrypt_key: Some("feishu-encrypt-key".to_string()),
            verification_token: Some("feishu-verify-token".to_string()),
            allowed_users: vec!["*".to_string()],
            group_reply: None,
            receive_mode: LarkReceiveMode::Webhook,
            port: Some(42617),
            draft_update_interval_ms:
                crate::app::agent::config::schema::default_lark_draft_update_interval_ms(),
            max_draft_edits: crate::app::agent::config::schema::default_lark_max_draft_edits(),
        });

        let masked = mask_sensitive_fields(&cfg);
        assert_eq!(masked.proxy.http_proxy.as_deref(), Some(MASKED_SECRET));
        assert_eq!(masked.proxy.https_proxy.as_deref(), Some(MASKED_SECRET));
        assert_eq!(masked.proxy.all_proxy.as_deref(), Some(MASKED_SECRET));
        assert_eq!(
            masked.tunnel.cloudflare.as_ref().map(|value| value.token.as_str()),
            Some(MASKED_SECRET)
        );
        assert_eq!(
            masked.tunnel.ngrok.as_ref().map(|value| value.auth_token.as_str()),
            Some(MASKED_SECRET)
        );
        assert_eq!(
            masked.channels_config.wati.as_ref().map(|value| value.api_token.as_str()),
            Some(MASKED_SECRET)
        );
        assert_eq!(
            masked.channels_config.email.as_ref().map(|value| value.password.as_str()),
            Some(MASKED_SECRET)
        );
        let masked_feishu =
            masked.channels_config.feishu.as_ref().expect("feishu config should exist");
        assert_eq!(masked_feishu.app_secret, MASKED_SECRET);
        assert_eq!(masked_feishu.encrypt_key.as_deref(), Some(MASKED_SECRET));
        assert_eq!(masked_feishu.verification_token.as_deref(), Some(MASKED_SECRET));
    }

    #[test]
    fn hydrate_config_for_save_restores_wati_email_and_feishu_secrets() {
        let mut current = crate::app::agent::config::Config::default();
        current.proxy.http_proxy = Some("http://user:pass@proxy.internal:8080".to_string());
        current.proxy.https_proxy = Some("https://user:pass@proxy.internal:8443".to_string());
        current.proxy.all_proxy = Some("socks5://user:pass@proxy.internal:1080".to_string());
        current.tunnel.cloudflare =
            Some(CloudflareTunnelConfig { token: "cloudflare-real-token".to_string() });
        current.tunnel.ngrok = Some(NgrokTunnelConfig {
            auth_token: "ngrok-real-token".to_string(),
            domain: Some("vibewindow.ngrok.app".to_string()),
        });
        current.channels_config.wati = Some(WatiConfig {
            api_token: "wati-real-token".to_string(),
            api_url: "https://live-mt-server.wati.io".to_string(),
            tenant_id: Some("tenant-1".to_string()),
            allowed_numbers: vec!["*".to_string()],
        });
        let mut email = crate::app::agent::channels::email_channel::EmailConfig::default();
        email.password = "email-real-password".to_string();
        current.channels_config.email = Some(email);
        current.channels_config.feishu = Some(crate::app::agent::config::FeishuConfig {
            app_id: "cli_app_id".to_string(),
            app_secret: "feishu-real-secret".to_string(),
            encrypt_key: Some("feishu-encrypt-key".to_string()),
            verification_token: Some("feishu-verify-token".to_string()),
            allowed_users: vec!["*".to_string()],
            group_reply: None,
            receive_mode: LarkReceiveMode::Webhook,
            port: Some(42617),
            draft_update_interval_ms:
                crate::app::agent::config::schema::default_lark_draft_update_interval_ms(),
            max_draft_edits: crate::app::agent::config::schema::default_lark_max_draft_edits(),
        });

        let incoming = mask_sensitive_fields(&current);
        let restored = hydrate_config_for_save(incoming, &current);

        assert_eq!(
            restored.proxy.http_proxy.as_deref(),
            Some("http://user:pass@proxy.internal:8080")
        );
        assert_eq!(
            restored.proxy.https_proxy.as_deref(),
            Some("https://user:pass@proxy.internal:8443")
        );
        assert_eq!(
            restored.proxy.all_proxy.as_deref(),
            Some("socks5://user:pass@proxy.internal:1080")
        );
        assert_eq!(
            restored.tunnel.cloudflare.as_ref().map(|value| value.token.as_str()),
            Some("cloudflare-real-token")
        );
        assert_eq!(
            restored.tunnel.ngrok.as_ref().map(|value| value.auth_token.as_str()),
            Some("ngrok-real-token")
        );
        assert_eq!(
            restored.channels_config.wati.as_ref().map(|value| value.api_token.as_str()),
            Some("wati-real-token")
        );
        assert_eq!(
            restored.channels_config.email.as_ref().map(|value| value.password.as_str()),
            Some("email-real-password")
        );
        let restored_feishu =
            restored.channels_config.feishu.as_ref().expect("feishu config should exist");
        assert_eq!(restored_feishu.app_secret, "feishu-real-secret");
        assert_eq!(restored_feishu.encrypt_key.as_deref(), Some("feishu-encrypt-key"));
        assert_eq!(restored_feishu.verification_token.as_deref(), Some("feishu-verify-token"));
    }

    #[test]
    fn integration_settings_payload_includes_openrouter_and_revision() {
        let config = crate::app::agent::config::Config::default();
        let payload = build_integration_settings_payload(&config);

        assert!(
            !payload.revision.is_empty(),
            "settings payload should include deterministic revision"
        );
        assert!(
            payload
                .integrations
                .iter()
                .any(|entry| entry.id == "openrouter" && entry.name == "OpenRouter"),
            "dashboard settings payload should expose OpenRouter editor metadata"
        );
    }

    #[test]
    fn apply_integration_credentials_update_switches_provider_with_fallback_model() {
        let mut config = crate::app::agent::config::Config::default();
        config.default_provider = Some("openrouter".to_string());
        config.default_model = Some("anthropic/claude-sonnet-4-6".to_string());
        config.api_url = Some("https://old.example.com".to_string());

        let updated = apply_integration_credentials_update(&config, "ollama", &BTreeMap::new())
            .expect("ollama update should succeed");

        assert_eq!(updated.default_provider.as_deref(), Some("ollama"));
        assert_eq!(updated.default_model.as_deref(), Some("llama3.2"));
        assert!(
            updated.api_url.is_none(),
            "switching providers without api_url field should reset stale api_url"
        );
    }

    #[test]
    fn apply_integration_credentials_update_rejects_unknown_fields() {
        let config = crate::app::agent::config::Config::default();
        let mut fields = BTreeMap::new();
        fields.insert("unknown".to_string(), "value".to_string());

        let err = apply_integration_credentials_update(&config, "openrouter", &fields)
            .expect_err("unknown fields should fail validation");
        assert!(err.contains("Unsupported field 'unknown'"));
    }

    #[test]
    fn config_revision_changes_when_config_changes() {
        let mut config = crate::app::agent::config::Config::default();
        let initial = config_revision(&config);
        config.default_model = Some("gpt-5.2".to_string());
        let changed = config_revision(&config);
        assert_ne!(initial, changed);
    }
}
