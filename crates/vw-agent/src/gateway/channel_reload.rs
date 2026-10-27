//! 网关通道快照构造与数据库配置重载。

use super::hash_webhook_secret;
use super::state::{AppState, GatewayChannels};
use crate::app::agent::channels::{
    LinqChannel, NextcloudTalkChannel, QQChannel, WatiChannel, WhatsAppChannel,
};
use crate::app::agent::config::{ChannelsConfig, Config};
use std::sync::Arc;

pub(super) fn build_gateway_channels(config: &ChannelsConfig) -> GatewayChannels {
    let config = config.available_channels();
    let webhook_secret_hash: Option<Arc<str>> = config.webhook.as_ref().and_then(|webhook| {
        webhook.secret.as_ref().and_then(|raw_secret| {
            let trimmed_secret = raw_secret.trim();
            (!trimmed_secret.is_empty())
                .then(|| Arc::<str>::from(hash_webhook_secret(trimmed_secret)))
        })
    });

    let whatsapp_channel: Option<Arc<WhatsAppChannel>> =
        config.whatsapp.as_ref().filter(|wa| wa.is_cloud_config()).map(|wa| {
            Arc::new(WhatsAppChannel::new(
                wa.access_token.clone().unwrap_or_default(),
                wa.phone_number_id.clone().unwrap_or_default(),
                wa.verify_token.clone().unwrap_or_default(),
                wa.allowed_numbers.clone(),
            ))
        });

    let whatsapp_app_secret: Option<Arc<str>> = std::env::var("VIBEWINDOW_WHATSAPP_APP_SECRET")
        .ok()
        .and_then(|secret| {
            let secret = secret.trim();
            (!secret.is_empty()).then(|| secret.to_owned())
        })
        .or_else(|| {
            config.whatsapp.as_ref().and_then(|wa| {
                wa.app_secret
                    .as_deref()
                    .map(str::trim)
                    .filter(|secret| !secret.is_empty())
                    .map(ToOwned::to_owned)
            })
        })
        .map(Arc::from);

    let linq_channel: Option<Arc<LinqChannel>> = config.linq.as_ref().map(|lq| {
        Arc::new(LinqChannel::new(
            lq.api_token.clone(),
            lq.from_phone.clone(),
            lq.allowed_senders.clone(),
        ))
    });

    let linq_signing_secret: Option<Arc<str>> = std::env::var("VIBEWINDOW_LINQ_SIGNING_SECRET")
        .ok()
        .and_then(|secret| {
            let secret = secret.trim();
            (!secret.is_empty()).then(|| secret.to_owned())
        })
        .or_else(|| {
            config.linq.as_ref().and_then(|lq| {
                lq.signing_secret
                    .as_deref()
                    .map(str::trim)
                    .filter(|secret| !secret.is_empty())
                    .map(ToOwned::to_owned)
            })
        })
        .map(Arc::from);

    let wati_channel: Option<Arc<WatiChannel>> = config.wati.as_ref().map(|wati_cfg| {
        Arc::new(WatiChannel::new(
            wati_cfg.api_token.clone(),
            wati_cfg.api_url.clone(),
            wati_cfg.tenant_id.clone(),
            wati_cfg.allowed_numbers.clone(),
        ))
    });

    let qq_channel: Option<Arc<QQChannel>> = config.qq.as_ref().map(|qq_cfg| {
        Arc::new(QQChannel::new(
            qq_cfg.app_id.clone(),
            qq_cfg.app_secret.clone(),
            qq_cfg.allowed_users.clone(),
        ))
    });
    let qq_webhook_enabled = config.qq.as_ref().is_some_and(|qq| {
        qq.receive_mode == crate::app::agent::config::schema::QQReceiveMode::Webhook
    });

    let nextcloud_talk_channel: Option<Arc<NextcloudTalkChannel>> =
        config.nextcloud_talk.as_ref().map(|nc| {
            Arc::new(NextcloudTalkChannel::new(
                nc.base_url.clone(),
                nc.app_token.clone(),
                nc.allowed_users.clone(),
            ))
        });

    let nextcloud_talk_webhook_secret: Option<Arc<str>> =
        std::env::var("VIBEWINDOW_NEXTCLOUD_TALK_WEBHOOK_SECRET")
            .ok()
            .and_then(|secret| {
                let secret = secret.trim();
                (!secret.is_empty()).then(|| secret.to_owned())
            })
            .or_else(|| {
                config.nextcloud_talk.as_ref().and_then(|nc| {
                    nc.webhook_secret
                        .as_deref()
                        .map(str::trim)
                        .filter(|secret| !secret.is_empty())
                        .map(ToOwned::to_owned)
                })
            })
            .map(Arc::from);

    GatewayChannels {
        webhook_disabled: config.unavailable_channels.contains("webhook:default"),
        runtimes: Default::default(),
        webhook_secret_hash,
        whatsapp: whatsapp_channel,
        whatsapp_app_secret,
        linq: linq_channel,
        linq_signing_secret,
        wati: wati_channel,
        qq: qq_channel,
        qq_webhook_enabled,
        nextcloud_talk: nextcloud_talk_channel,
        nextcloud_talk_webhook_secret,
    }
}

pub(super) async fn watch_gateway_channels(state: AppState, config: Config) -> anyhow::Result<()> {
    let source =
        crate::app::agent::config::schema::ChannelConfigWatcher::new(&config, false).await?;
    watch_gateway_channels_with_source(state, config, source).await
}

pub(super) async fn watch_gateway_channels_with_source(
    state: AppState,
    mut config: Config,
    mut source: crate::app::agent::config::schema::ChannelConfigWatcher,
) -> anyhow::Result<()> {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        match source.poll(&config).await {
            Ok(Some(channels)) => {
                let mut snapshot = build_gateway_channels(&channels);
                let mut current = state.channels.lock();
                snapshot.runtimes = current.runtimes.clone();
                for runtime in snapshot.runtimes.lock().values() {
                    runtime.project_state.lock().unwrap_or_else(|e| e.into_inner()).config =
                        channels.clone();
                }
                *current = snapshot;
                drop(current);
                config.channels_config = channels;
                state.config.lock().channels_config = config.channels_config.clone();
                tracing::info!("Reloaded gateway channel configuration");
            }
            Ok(None) => {}
            Err(_) => {
                tracing::warn!("Unable to reload gateway channels; keeping current configuration")
            }
        }
    }
}

#[cfg(test)]
#[path = "channel_reload_tests.rs"]
mod tests;
