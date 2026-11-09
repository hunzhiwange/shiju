use serde_json::json;
use vw_gateway_client::{AppsUsageDisplay, AppsUsageSettings};

use super::{Draft, Field, Kind};

const AK: &str = "a0a0a0a0-0000-4000-8000-000000000001";
const SK: &str = "a0a0a0a0-0000-4000-8000-000000000002";

#[test]
fn usage_usage_settings_returns_expected_result() {
    let mut draft = Draft::new(None);
    draft.select(Kind::ZhipuTeam);
    draft.edit(Field::Organization, " org-a ".into());
    draft.edit(Field::Project, " project-a ".into());
    draft.edit(Field::Interval, "12".into());
    draft.display = AppsUsageDisplay::Used;
    let settings = draft.settings().unwrap();
    assert_eq!(
        serde_json::to_value(&settings).unwrap(),
        json!({
            "template": "token_plan",
            "enabled": true,
            "refreshIntervalMinutes": 12,
            "display": "used",
            "credentials": {},
            "codingPlan": {"kind": "zhipu_team", "organizationId": "org-a", "projectId": "project-a"}
        })
    );

    draft.select(Kind::Volcengine);
    assert!(!draft.enabled);
    draft.edit(Field::AccessKey, AK.into());
    draft.edit(Field::SecretKey, SK.into());
    let saved = draft.settings().unwrap();
    let reopened: AppsUsageSettings =
        serde_json::from_value(serde_json::to_value(&saved).unwrap()).unwrap();
    let mut reopened = Draft::new(Some(reopened));
    assert_eq!(reopened.settings().unwrap(), saved);
    reopened.enabled = true;
    assert!(reopened.settings().unwrap().enabled);
}

#[test]
fn usage_usage_settings_rejects_invalid_input() {
    let mut draft = Draft::new(None);
    assert!(draft.settings().is_err());

    draft.select(Kind::ZhipuTeam);
    draft.edit(Field::Organization, "org-a".into());
    draft.edit(Field::Project, "   ".into());
    assert!(draft.settings().is_err());
    draft.edit(Field::Project, "project-a".into());
    draft.edit(Field::Interval, "1441".into());
    assert!(draft.settings().is_err());

    draft.edit(Field::Interval, "0".into());
    draft.select(Kind::Volcengine);
    draft.enabled = true;
    draft.edit(Field::AccessKey, AK.into());
    assert!(draft.settings().is_err());
    draft.edit(Field::SecretKey, "raw-secret-fixture".into());
    assert!(draft.settings().is_err());
}

#[tokio::test]
async fn usage_usage_settings_save_button_requires_team_project() {
    use crate::app::{
        Message,
        components::apps_usage_details_tests::{Scene, renderer},
        message::{SettingsMessage, app_providers::list::ListMessage},
    };
    use iced::{Event as IcedEvent, Length, mouse, widget::Space};
    use vw_gateway_client::vw_api_types::{agent_apps::AppId, apps_usage::UsageSnapshotQuery};

    use super::{Event, Phase, view::modal_view};

    let selection = UsageSnapshotQuery {
        project_id: "project-a".into(),
        target_host_id: "local".into(),
        app_id: AppId::Codex,
        provider_id: "provider-a".into(),
        account_id: None,
        kind: None,
    };
    let mut renderer = renderer().await;
    let mut draft = Draft::new(None);
    draft.select(Kind::ZhipuTeam);
    draft.edit(Field::Organization, "org-a".into());
    let click = |scene: &mut Scene<'_>, renderer: &mut iced::Renderer| {
        let position = scene.bounds("保存").unwrap().center();
        scene.send(
            renderer,
            "保存",
            &[
                IcedEvent::Mouse(mouse::Event::CursorMoved { position }),
                IcedEvent::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                IcedEvent::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
        )
    };
    let base = || Space::new().width(Length::Fill).height(Length::Fill).into();
    let incomplete = Phase::Editing { draft: draft.clone(), saving: false };
    let view = modal_view(&incomplete, &selection, base());
    let mut scene = Scene::new(view, &mut renderer);
    assert!(click(&mut scene, &mut renderer).is_empty());

    draft.edit(Field::Project, "team-project".into());
    let complete = Phase::Editing { draft, saving: false };
    let view = modal_view(&complete, &selection, base());
    let mut scene = Scene::new(view, &mut renderer);
    let messages = click(&mut scene, &mut renderer);
    assert!(
        matches!(
            messages.as_slice(),
            [Message::Settings(SettingsMessage::AppProviders(ListMessage::UsageSettings(
                Event::Save
            )))]
        ),
        "{messages:?}"
    );
}
