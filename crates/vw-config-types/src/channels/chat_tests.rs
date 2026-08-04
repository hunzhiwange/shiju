use super::ChannelsConfig;

#[test]
fn task_614_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("chat_tests.rs"));
}

fn matrix_instance(id: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "homeserver": "https://matrix.example",
        "access_token": "matrix-token",
        "room_id": "!room:example",
        "allowed_users": []
    })
}

fn feishu_instance(id: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "app_id": "feishu-app",
        "app_secret": "feishu-secret"
    })
}

fn dingtalk_instance(id: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "client_id": "dingtalk-client",
        "client_secret": "dingtalk-secret"
    })
}

#[test]
fn channel_instance_profiles_resolve_legacy_and_named_instances() {
    let channels: ChannelsConfig = serde_json::from_value(serde_json::json!({
        "cli": true,
        "matrix_enabled": true,
        "matrix": {
            "homeserver": "https://matrix-default.example",
            "access_token": "matrix-default-token",
            "room_id": "!default:example",
            "allowed_users": []
        },
        "matrix_instances": [
            matrix_instance("planner")
        ],
        "feishu": {
            "app_id": "feishu-default-app",
            "app_secret": "feishu-default-secret"
        },
        "feishu_instances": [
            feishu_instance("cn")
        ],
        "dingtalk": {
            "client_id": "dingtalk-default-client",
            "client_secret": "dingtalk-default-secret"
        },
        "dingtalk_instances": [
            dingtalk_instance("1")
        ]
    }))
    .unwrap();

    let profiles = channels.resolve_channel_instance_profiles().unwrap();
    let refs: Vec<_> = profiles.iter().map(|profile| profile.profile_ref.as_str()).collect();

    assert_eq!(
        refs,
        vec![
            "matrix:default",
            "matrix:planner",
            "feishu:default",
            "feishu:cn",
            "dingtalk:default",
            "dingtalk:1"
        ]
    );
}

#[test]
fn channel_instance_profiles_reject_duplicate_or_unsafe_ids() {
    let cases = [
        (
            "empty",
            serde_json::json!({
                "cli": true,
                "matrix_enabled": true,
                "matrix_instances": [matrix_instance("")]
            }),
            "must not be empty",
        ),
        (
            "colon",
            serde_json::json!({
                "cli": true,
                "matrix_enabled": true,
                "matrix_instances": [matrix_instance("bad:id")]
            }),
            "must not contain",
        ),
        (
            "slash",
            serde_json::json!({
                "cli": true,
                "feishu_instances": [feishu_instance("bad/id")]
            }),
            "must not contain",
        ),
        (
            "backslash",
            serde_json::json!({
                "cli": true,
                "dingtalk_instances": [dingtalk_instance("bad\\id")]
            }),
            "must not contain",
        ),
        (
            "duplicate",
            serde_json::json!({
                "cli": true,
                "matrix_enabled": true,
                "matrix": {
                    "homeserver": "https://matrix-default.example",
                    "access_token": "matrix-default-token",
                    "room_id": "!default:example",
                    "allowed_users": []
                },
                "matrix_instances": [matrix_instance("default")]
            }),
            "duplicated",
        ),
    ];

    for (name, payload, expected) in cases {
        let channels: ChannelsConfig = serde_json::from_value(payload).unwrap();
        let err = channels.resolve_channel_instance_profiles().unwrap_err().to_string();
        assert!(err.contains(expected), "{name}: expected {err:?} to contain {expected:?}");
    }
}

#[test]
fn matrix_is_disabled_by_default_and_disabled_profiles_are_ignored() {
    let channels: ChannelsConfig = serde_json::from_value(serde_json::json!({
        "cli": true,
        "matrix_instances": [matrix_instance("bad:id")]
    }))
    .unwrap();

    assert!(!channels.matrix_enabled);
    assert!(channels.resolve_channel_instance_profiles().unwrap().is_empty());
}
