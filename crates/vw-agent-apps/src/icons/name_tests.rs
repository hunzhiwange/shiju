use super::name::normalize_icon_name;
use vw_api_types::agent_apps::AgentAppsErrorCode;

#[test]
fn cc_base_icon_name_trims_surrounding_whitespace() {
    assert_eq!(normalize_icon_name(" \t图标 名称\n　"), Ok("图标 名称".to_owned()));
}

#[test]
fn cc_base_icon_name_rejects_empty() {
    assert_eq!(normalize_icon_name(""), Err(AgentAppsErrorCode::InvalidInput));
}

#[test]
fn cc_base_icon_name_rejects_whitespace() {
    assert_eq!(normalize_icon_name(" \t\n　"), Err(AgentAppsErrorCode::InvalidInput));
}

#[test]
fn cc_base_icon_name_accepts_64_characters() {
    for name in ["a".repeat(64), "图".repeat(64)] {
        assert_eq!(normalize_icon_name(&format!(" {name} ")), Ok(name));
    }
}

#[test]
fn cc_base_icon_name_rejects_65_characters() {
    for name in ["a".repeat(65), "图".repeat(65)] {
        assert_eq!(normalize_icon_name(&name), Err(AgentAppsErrorCode::InvalidInput));
    }
}
