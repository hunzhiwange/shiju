use agent_config::runtime::Config;
use agent_cron::{
    DeliveryConfig,
    delivery::{self, DeliveryHost},
};
use serde::Deserialize;
use serde_json::{Value, json};

pub(super) fn delivery_schema(update: bool) -> Value {
    let mut schema = json!({
        "type": "object",
        "properties": {
            "mode": {"type": "string", "enum": ["none", "announce"], "description": "Set to announce to deliver task output"},
            "channel": {"type": "string", "enum": ["telegram", "discord", "slack", "mattermost", "qq", "email", "wecom", "wecom_cron"]},
            "to": {"type": "string", "description": "Destination ID. For wecom use a userid/chatid; for wecom_cron omit to for the default bound bot or use a cron bot instance ID."},
            "best_effort": {"type": "boolean", "description": "If true, delivery failure does not fail the task"}
        }
    });
    if update {
        schema["description"] = json!(
            "Change delivery options. Omit channel and to to preserve an existing task Webhook while adjusting mode or best_effort."
        );
    }
    schema
}

#[derive(Deserialize)]
pub(super) struct DeliveryPatch {
    mode: Option<String>,
    channel: Option<String>,
    to: Option<String>,
    best_effort: Option<bool>,
}

impl DeliveryPatch {
    pub(super) async fn apply(
        self,
        mut delivery: DeliveryConfig,
        config: &Config,
        host: Option<&dyn DeliveryHost>,
    ) -> anyhow::Result<DeliveryConfig> {
        if let Some(mode) = self.mode {
            delivery.mode = nonempty(mode).unwrap_or_else(|| "none".into());
        }
        if let Some(channel) = self.channel {
            if !delivery
                .channel
                .as_deref()
                .is_some_and(|old| old.eq_ignore_ascii_case(channel.trim()))
            {
                delivery.to = None;
            }
            delivery.channel = nonempty(channel);
        }
        if let Some(to) = self.to {
            if delivery
                .channel
                .as_deref()
                .is_some_and(|channel| channel.eq_ignore_ascii_case("wecom_webhook"))
                || delivery::is_task_webhook(to.trim())
                || to.contains("qyapi.weixin.qq.com/cgi-bin/webhook/")
            {
                anyhow::bail!("Configure the task Webhook in Scheduled Tasks settings");
            }
            delivery.to = nonempty(to);
        }
        if let Some(best_effort) = self.best_effort {
            delivery.best_effort = best_effort;
        }
        if let Some(host) = host {
            delivery::validate_saved(host, config, &delivery).await?;
        } else if delivery.mode.eq_ignore_ascii_case("announce") {
            delivery::validate_target(&delivery)?;
        }
        Ok(delivery)
    }
}

fn nonempty(value: String) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}
