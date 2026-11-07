use super::*;
use crate::app::state::{AgentAppsContext, IconRenameState};
use iced::advanced::widget::{Id, operation::focusable};
use vw_gateway_client::vw_api_types::id::TenantScope;

fn rename_form() -> IconRenameState {
    IconRenameState {
        id: "icon-fixture".into(),
        name: NAME.into(),
        saving: false,
        error: Some("名称校验失败 · 名稱驗證失敗 · Name validation failed".into()),
        context: AgentAppsContext {
            gateway: "http://fixture.invalid".into(),
            tenant: TenantScope::new("fixture-platform", "fixture-company").unwrap(),
            project: None,
        },
        generation: 1,
    }
}

fn base(state: &AgentAppsState) -> Element<'_, Message> {
    container(view(state)).width(Length::Fill).height(Length::Fill).into()
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_24_icon_rows_and_rename_overlay() {
    if !isolated("rename_tests::ui_refresh_24_icon_rows_and_rename_overlay") {
        return;
    }
    use iced::keyboard::key::Named;

    let mut renderer = renderer().await;
    let mut state = fixture().agent_apps;
    let long_name = format!("{}{}", "简体中文繁體中文".repeat(6), "LongEnglishName".repeat(10));
    state.icons.insert(0, CustomIconEntry {
        id: "long-icon".into(),
        name: long_name.clone(),
        data: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAABytg0kAAAAFElEQVR4nGMwLv//X2n3zP8MMAYAV4cKO3iHzecAAAAASUVORK5CYII=".into(),
        created_at: 0,
    });
    let size = Size::new(640, 310);
    let mut ui = scene(view(&state), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let rename = positions
        .labels
        .iter()
        .filter(|(label, _)| label == "改名")
        .map(|(_, bounds)| *bounds)
        .collect::<Vec<_>>();
    assert_eq!(rename.len(), 2);
    let long = positions.label(&long_name);
    assert_fits(long, size);
    assert!(long.x + long.width < rename[0].x);
    assert!((long.center_y() - rename[0].center_y()).abs() < 0.5);
    assert!(matches!(
        click(&mut ui, &mut renderer, rename[1].center()).as_slice(),
        [Message::Settings(SettingsMessage::AgentApps(AgentAppsMessage::RenameOpen(id)))]
            if id == "icon-fixture"
    ));
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Light,
        size,
        "icons-640-light",
        cursor(rename[1].center()),
    );
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "icons-640-dark",
        cursor(rename[0].center()),
    );
    drop(ui);

    state.icons.clear();
    snapshot(view(&state), &mut renderer, &Theme::Light, size, "icons-empty-light");
    state.loading = true;
    let mut ui = scene(view(&state), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    assert_fits(positions.label("正在加载图标…"), size);
    assert!(click(&mut ui, &mut renderer, positions.label("刷新").center()).is_empty());
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "icons-loading-dark",
        mouse::Cursor::Unavailable,
    );
    drop(ui);

    state.loading = false;
    state.error = Some("图标服务不可用 · 圖標服務不可用 · Fixture icon service unavailable".into());
    let mut ui = scene(view(&state), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    assert_fits(positions.label(state.error.as_deref().unwrap()), size);
    let retry = positions.label("重试").center();
    assert!(matches!(
        click(&mut ui, &mut renderer, retry).as_slice(),
        [Message::Settings(SettingsMessage::AgentApps(AgentAppsMessage::Refresh))]
    ));
    capture(&mut ui, &mut renderer, &Theme::Light, size, "icons-error-light", cursor(retry));
    capture(&mut ui, &mut renderer, &Theme::Dark, size, "icons-error-dark", cursor(retry));
    drop(ui);

    state.error = None;
    let size = Size::new(420, 450);
    let mut background = scene(base(&state), &mut renderer, size);
    let refresh = geometry(&mut background, &renderer).label("刷新").center();
    drop(background);
    state.rename = Some(rename_form());
    let mut ui = scene(view_overlays(&state, base(&state)), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let (viewport, _) = positions.scroll.unwrap();
    let input = positions.input(NAME);
    let save = positions.label("保存");
    let cancel = positions.label("取消");
    assert_fits(input, size);
    assert_fits(save, size);
    assert!(viewport.contains(input.center()));
    assert!(save.y > viewport.y + viewport.height);
    for (theme, tag) in [(&Theme::Light, "light"), (&Theme::Dark, "dark")] {
        capture(
            &mut ui,
            &mut renderer,
            theme,
            size,
            &format!("rename-420-{tag}"),
            mouse::Cursor::Unavailable,
        );
    }
    ui.operate(&renderer, &mut focusable::focus(Id::new("custom-icon-rename")));
    let input_cursor = cursor(input.center());
    assert!(matches!(
        send(&mut ui, &mut renderer, input_cursor, &[key(Named::Enter)]).as_slice(),
        [Message::Settings(SettingsMessage::AgentApps(AgentAppsMessage::RenameSubmit))]
    ));
    assert!(send(&mut ui, &mut renderer, input_cursor, &[key(Named::Escape)]).is_empty());
    assert!(matches!(
        click(&mut ui, &mut renderer, cancel.center()).as_slice(),
        [Message::Settings(SettingsMessage::AgentApps(AgentAppsMessage::RenameClose))]
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, save.center()).as_slice(),
        [Message::Settings(SettingsMessage::AgentApps(AgentAppsMessage::RenameSubmit))]
    ));
    assert!(click(&mut ui, &mut renderer, input.center()).is_empty());
    assert!(send(&mut ui, &mut renderer, input_cursor, &[key(Named::End)]).is_empty());
    assert!(matches!(
        send(&mut ui, &mut renderer, input_cursor, &[key(Named::Backspace)]).as_slice(),
        [Message::Settings(SettingsMessage::AgentApps(AgentAppsMessage::RenameInput(value)))]
            if value == "工作图标 · 測試 · Ico"
    ));
    // The refresh button below the dialog cannot react to pointer events.
    move_mouse(&mut ui, &mut renderer, refresh);
    assert!(click(&mut ui, &mut renderer, refresh).is_empty());
    drop(ui);

    let form = state.rename.as_mut().unwrap();
    form.name.clear();
    form.error = Some("/fixture/示例/測試/".repeat(48).replace("/fixture", "\n/fixture"));
    let mut ui = scene(view_overlays(&state, base(&state)), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let input = positions.input("请输入图标名称");
    assert!(click(&mut ui, &mut renderer, input.center()).is_empty());
    assert!(
        matches!(
            send(&mut ui, &mut renderer, cursor(input.center()), &[key(Named::Enter)]).as_slice(),
            [Message::Settings(SettingsMessage::AgentApps(AgentAppsMessage::RenameSubmit))]
        ),
        "empty input is still submitted for gateway-side validation"
    );
    let (viewport, initial) = positions.scroll.unwrap();
    assert!(
        send(
            &mut ui,
            &mut renderer,
            cursor(viewport.center()),
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -300.0 }
            })],
        )
        .is_empty()
    );
    let scrolled = geometry(&mut ui, &renderer);
    assert!(scrolled.scroll.unwrap().1.y > initial.y);
    assert_eq!(positions.label("保存"), scrolled.label("保存"));
    assert_eq!(positions.label("图标改名"), scrolled.label("图标改名"));
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "rename-error-scrolled-dark",
        mouse::Cursor::Unavailable,
    );
    drop(ui);

    let form = state.rename.as_mut().unwrap();
    form.saving = true;
    form.error = None;
    let mut ui = scene(view_overlays(&state, base(&state)), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    for label in ["保存中…", "取消"] {
        assert!(click(&mut ui, &mut renderer, positions.label(label).center()).is_empty());
    }
    let point = positions.input("请输入图标名称").center();
    assert!(click(&mut ui, &mut renderer, point).is_empty());
    assert!(send(&mut ui, &mut renderer, cursor(point), &[key(Named::Enter)]).is_empty());
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "rename-saving-dark",
        mouse::Cursor::Unavailable,
    );
}
