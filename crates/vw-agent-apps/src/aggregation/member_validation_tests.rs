use super::member_validation::{
    MemberValidationError::{DefaultMemberRequired, InvalidMember, UnsupportedApp},
    ProviderRole::{Default as DefaultRole, Member},
    validate_member_removal, validate_provider,
};
use crate::providers::{
    live::commit::{AppState, Mode, ModeState},
    mode_state::StackState,
};
use vw_api_types::{agent_apps::AppId, agent_apps_auth::AuthService};

#[test]
fn aggregation_t05_member_validation_preserves_contract() {
    let mut state = AppState {
        stack: StackState {
            enabled: true,
            members: vec!["current".into(), "other".into()],
            ..Default::default()
        },
        mode: Some(ModeState {
            mode: Mode::Proxy,
            attached: true,
            proxy_route: Some("current".into()),
            contract: None,
        }),
        ..Default::default()
    };
    assert_eq!(
        validate_member_removal(AppId::Claude, &state, "current"),
        Err(DefaultMemberRequired)
    );
    assert_eq!(validate_member_removal(AppId::Claude, &state, "other"), Ok(()));
    assert_eq!(validate_member_removal(AppId::Claude, &state, "missing"), Ok(()));
    let mode = state.mode.as_mut().unwrap();
    mode.proxy_route = Some("other".into());
    assert_eq!(validate_member_removal(AppId::Claude, &state, "current"), Ok(()));
    state.stack.enabled = false;
    assert_eq!(validate_member_removal(AppId::Claude, &state, "other"), Ok(()));
    state.stack.enabled = true;
    state.mode.as_mut().unwrap().mode = Mode::Direct;
    assert_eq!(validate_member_removal(AppId::Claude, &state, "other"), Ok(()));
}

#[test]
fn aggregation_t05_official_accounts_are_limited_to_codex_default() {
    for app in [AppId::Claude, AppId::Codex, AppId::ClaudeDesktop] {
        for role in [Member, DefaultRole] {
            assert_eq!(validate_provider(app, None, role), Ok(()));
            assert_eq!(
                validate_provider(app, Some(AuthService::ClaudeOfficial), role),
                Err(InvalidMember)
            );
            let expected = if (app, role) == (AppId::Codex, DefaultRole) {
                Ok(())
            } else {
                Err(InvalidMember)
            };
            assert_eq!(validate_provider(app, Some(AuthService::CodexOauth), role), expected);
        }
    }
}

#[test]
fn aggregation_t05_unsupported_apps_are_rejected_for_every_action() {
    let app = AppId::Gemini;
    assert_eq!(validate_provider(app, None, Member), Err(UnsupportedApp));
    assert_eq!(
        validate_provider(app, Some(AuthService::CodexOauth), DefaultRole),
        Err(UnsupportedApp)
    );
    assert_eq!(validate_member_removal(app, &AppState::default(), "any"), Err(UnsupportedApp));
}
