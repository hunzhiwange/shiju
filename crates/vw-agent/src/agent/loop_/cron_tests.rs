use super::*;

#[test]
fn injects_delivery_for_supported_agent_cron() {
    let mut args = serde_json::json!({"prompt": "check status"});

    maybe_inject_cron_add_delivery("cron_add", &mut args, "telegram", Some("chat-1"));

    assert_eq!(args["delivery"]["mode"], "announce");
    assert_eq!(args["delivery"]["channel"], "telegram");
    assert_eq!(args["delivery"]["to"], "chat-1");
}

#[test]
fn injects_delivery_for_explicit_agent_cron_without_prompt() {
    let mut args = serde_json::json!({"job_type": "agent"});

    maybe_inject_cron_add_delivery("cron_add", &mut args, "slack", Some("C123"));

    assert_eq!(args["delivery"]["mode"], "announce");
    assert_eq!(args["delivery"]["channel"], "slack");
    assert_eq!(args["delivery"]["to"], "C123");
}

#[test]
fn fills_missing_delivery_fields_when_mode_is_announce() {
    let mut args = serde_json::json!({
        "prompt": "check status",
        "delivery": {
            "mode": "announce",
            "channel": "",
            "to": "  "
        }
    });

    maybe_inject_cron_add_delivery("cron_add", &mut args, "mattermost", Some("room-1"));

    assert_eq!(args["delivery"]["mode"], "announce");
    assert_eq!(args["delivery"]["channel"], "mattermost");
    assert_eq!(args["delivery"]["to"], "room-1");
}

#[test]
fn replaces_none_delivery_mode() {
    let mut args = serde_json::json!({
        "prompt": "check status",
        "delivery": {"mode": "NoNe"}
    });

    maybe_inject_cron_add_delivery("cron_add", &mut args, "discord", Some("channel-1"));

    assert_eq!(args["delivery"]["mode"], "announce");
    assert_eq!(args["delivery"]["channel"], "discord");
    assert_eq!(args["delivery"]["to"], "channel-1");
}

#[test]
fn replaces_blank_delivery_mode_without_overwriting_existing_target() {
    let mut args = serde_json::json!({
        "prompt": "check status",
        "delivery": {
            "mode": "  ",
            "channel": "discord",
            "to": "channel-1"
        }
    });

    maybe_inject_cron_add_delivery("cron_add", &mut args, "telegram", Some("chat-1"));

    assert_eq!(args["delivery"]["mode"], "announce");
    assert_eq!(args["delivery"]["channel"], "discord");
    assert_eq!(args["delivery"]["to"], "channel-1");
}

#[test]
fn preserves_explicit_non_announce_delivery_mode() {
    let mut args = serde_json::json!({
        "prompt": "check status",
        "delivery": {"mode": "silent"}
    });

    maybe_inject_cron_add_delivery("cron_add", &mut args, "telegram", Some("chat-1"));

    assert_eq!(args["delivery"]["mode"], "silent");
    assert!(args["delivery"].get("channel").is_none());
}

#[test]
fn skips_non_cron_add_tool() {
    let mut args = serde_json::json!({"prompt": "check status"});

    maybe_inject_cron_add_delivery("file_read", &mut args, "telegram", Some("chat-1"));

    assert!(args.get("delivery").is_none());
}

#[test]
fn skips_unsupported_channel() {
    let mut args = serde_json::json!({"prompt": "check status"});

    maybe_inject_cron_add_delivery("cron_add", &mut args, "web", Some("chat-1"));

    assert!(args.get("delivery").is_none());
}

#[test]
fn skips_missing_reply_target() {
    let mut args = serde_json::json!({"prompt": "check status"});

    maybe_inject_cron_add_delivery("cron_add", &mut args, "telegram", None);

    assert!(args.get("delivery").is_none());
}

#[test]
fn skips_blank_reply_target() {
    let mut args = serde_json::json!({"prompt": "check status"});

    maybe_inject_cron_add_delivery("cron_add", &mut args, "telegram", Some("  "));

    assert!(args.get("delivery").is_none());
}

#[test]
fn skips_non_object_args() {
    let mut args = serde_json::json!("not an object");

    maybe_inject_cron_add_delivery("cron_add", &mut args, "telegram", Some("chat-1"));

    assert_eq!(args, serde_json::json!("not an object"));
}

#[test]
fn skips_args_without_agent_job_signal() {
    let mut args = serde_json::json!({"command": "echo hello"});

    maybe_inject_cron_add_delivery("cron_add", &mut args, "telegram", Some("chat-1"));

    assert!(args.get("delivery").is_none());
}

#[test]
fn skips_explicit_non_agent_job_type_even_with_prompt() {
    let mut args = serde_json::json!({
        "job_type": "shell",
        "prompt": "check status"
    });

    maybe_inject_cron_add_delivery("cron_add", &mut args, "telegram", Some("chat-1"));

    assert!(args.get("delivery").is_none());
}

#[test]
fn skips_non_object_delivery() {
    let mut args = serde_json::json!({
        "prompt": "check status",
        "delivery": "announce"
    });

    maybe_inject_cron_add_delivery("cron_add", &mut args, "telegram", Some("chat-1"));

    assert_eq!(args["delivery"], "announce");
}
