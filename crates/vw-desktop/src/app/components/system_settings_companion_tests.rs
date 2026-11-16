// 宿主契约：真实 avatar_preview 的字体与精灵渲染、ViewMessage 更新窗口状态、
// 搜索导航路由，以及点击预览不触发窗口动作；不能用占位头像替代这些断言。
use super::*;
use crate::app::App;
use crate::app::components::system_settings_search::{self, SystemSettingsSearchTarget};
use crate::app::message::{settings, view};
use crate::app::state::{GeneralSettingsTab, TaskPetAvatarKind};

#[test]
fn mini_companion_settings_selection_updates_preview_and_window() {
    let mut app = App::new().0;
    for avatar in [TaskPetAvatarKind::Robot, TaskPetAvatarKind::Beauty, TaskPetAvatarKind::Handsome]
    {
        let _ = view::update(&mut app, view::ViewMessage::TaskPetAvatarSelected(avatar));
        assert_eq!(app.pet_state.task_pet_avatar_kind, avatar);
        std::hint::black_box(content(&app));
    }
}

#[test]
fn mini_companion_settings_none_keeps_mini_visible() {
    let mut app = App::new().0;
    app.companion_navigation.task_pet_visible = true;
    let id = iced::window::Id::unique();
    app.task_pet_window_id = Some(id);
    let _ =
        view::update(&mut app, view::ViewMessage::TaskPetAvatarSelected(TaskPetAvatarKind::None));
    assert!(app.companion_navigation.task_pet_visible);
    assert_eq!(app.task_pet_window_id, Some(id));
    std::hint::black_box(content(&app));
}

#[test]
fn mini_companion_settings_search_reaches_page() {
    let mut app = App::new().0;
    for query in ["伙伴", "companion", "mini"] {
        let entry = system_settings_search::catalog(&app)
            .into_iter()
            .find(|entry| {
                entry.matches(query)
                    && entry.target
                        == SystemSettingsSearchTarget::General(GeneralSettingsTab::Companion)
            })
            .expect("companion page must be searchable");
        let _ = settings::update(
            &mut app,
            settings::SettingsMessage::SystemSearchTargetSelected(entry.target),
        );
        assert_eq!(app.general_settings.general_settings_tab, GeneralSettingsTab::Companion);
    }
    let _ = view::update(&mut app, view::ViewMessage::TaskPetOpenSettings);
    assert!(app.navigation_panels.show_system_settings);
    assert_eq!(app.general_settings.general_settings_tab, GeneralSettingsTab::Companion);
}

#[path = "../../../../libs/desktop-settings-appearance/src/companion_test_support.rs"]
mod rendering;
use rendering::Scene;
async fn scene(app: &App, width: f32, click: Option<&str>) -> (Scene, Vec<crate::app::Message>) {
    rendering::scene(content(app), preview_id(app.pet_state.task_pet_avatar_kind), width, click).await
}

#[tokio::test]
async fn mini_companion_selection_events_preserve_window() {
    use crate::app::Message;
    let mut app = App::new().0;
    app.companion_navigation.task_pet_visible = true;
    let id = iced::window::Id::unique();
    app.task_pet_window_id = Some(id);
    for &(avatar, name, _) in &COMPANIONS {
        let (_, messages) = scene(&app, 800.0, Some(name)).await;
        for message in messages {
            if let Message::View(message) = message {
                let _ = view::update(&mut app, message);
            }
        }
        assert_eq!(app.pet_state.task_pet_avatar_kind, avatar);
        assert!(app.companion_navigation.task_pet_visible);
        assert_eq!(app.task_pet_window_id, Some(id));
    }
}
