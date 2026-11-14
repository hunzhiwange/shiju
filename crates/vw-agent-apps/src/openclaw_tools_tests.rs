use super::openclaw_tools::*;
use crate::providers::live::patch_json::object_value;
use serde_json::json;

#[test]
fn openclaw_tools_preserves_unknown_profile_and_extensions() {
    let source = br#"{
        // keep root comment
        "agents": {"defaults": {"workspace": "keep"}},
        "tools": {"profile": "future", "allow": ["", "  ", " shell:* "],
                  "deny": ["\t", "exec"], "future": {"enabled": false}}
    }"#;
    let tools = get_tools_config(Some(source)).unwrap().unwrap();
    assert_eq!(tools.profile_kind(), OpenClawToolsProfile::Unknown("future"));
    let next = prepare_tools_config(Some(source), &tools).unwrap();
    let result = get_tools_config(Some(&next)).unwrap().unwrap();
    assert_eq!(result.profile_kind(), OpenClawToolsProfile::Unknown("future"));
    assert_eq!(result.allow, vec![" shell:* "]);
    assert_eq!(result.deny, vec!["exec"]);
    assert_eq!(result.extra["future"], json!({"enabled": false}));
    assert_eq!(object_value(&next).unwrap()["agents"], object_value(source).unwrap()["agents"]);
    assert!(std::str::from_utf8(&next).unwrap().contains("// keep root comment"));
}

#[test]
fn openclaw_tools_preserves_unknown_profile_distinguishes_unset_and_known() {
    assert!(get_tools_config(None).unwrap().is_none());
    assert!(get_tools_config(Some(b"{}")).unwrap().is_none());
    for (profile, expected) in [
        (None, OpenClawToolsProfile::Unset),
        (Some("minimal"), OpenClawToolsProfile::Minimal),
        (Some("coding"), OpenClawToolsProfile::Coding),
        (Some("messaging"), OpenClawToolsProfile::Messaging),
        (Some("full"), OpenClawToolsProfile::Full),
        (Some(""), OpenClawToolsProfile::Unknown("")),
    ] {
        let tools = OpenClawToolsConfig { profile: profile.map(str::to_owned), ..Default::default() };
        let next = prepare_tools_config(None, &tools).unwrap();
        let read = get_tools_config(Some(&next)).unwrap().unwrap();
        assert_eq!(read.profile_kind(), expected);
    }
}

#[test]
fn openclaw_tools_preserves_unknown_profile_rejects_malformed_and_shadowed_fields() {
    for source in ["broken", "[]", r#"{"tools":null}"#, r#"{"tools":[]}"#,
        r#"{"tools":{"profile":42}}"#, r#"{"tools":{"allow":[1]}}"#] {
        assert!(get_tools_config(Some(source.as_bytes())).is_err());
        assert!(prepare_tools_config(Some(source.as_bytes()), &Default::default()).is_err());
    }
    for key in ["profile", "allow", "deny"] {
        let mut tools = OpenClawToolsConfig::default();
        tools.extra.insert(key.into(), json!("shadow"));
        assert!(prepare_tools_config(None, &tools).is_err());
    }
}
