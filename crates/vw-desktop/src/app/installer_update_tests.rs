use super::*;
use sha2::{Digest, Sha256};
use desktop_release::InstallerKind;

fn fixture(kind: InstallerKind) -> (tempfile::TempDir, PreparedDesktopUpdate) {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().canonicalize().unwrap();
    let extension = match kind {
        InstallerKind::Dmg => "dmg", InstallerKind::Msi => "msi",
        InstallerKind::Rpm => "rpm", InstallerKind::Deb => "deb",
    };
    let name = format!("installer.{extension}");
    let path = directory.join(&name);
    std::fs::write(&path, b"verified").unwrap();
    #[derive(serde::Deserialize)]
    struct Handoff {
        ready: PreparedDesktopUpdate,
    }
    let handoff: Handoff = serde_json::from_value(serde_json::json!({"ready": {
        "path": path, "directory": directory, "version": "1.2.3",
        "asset": {"name": name, "target": "", "kind": kind, "url": "",
            "size": 8, "sha256": hex::encode(Sha256::digest(b"verified"))}
    }})).unwrap();
    (root, handoff.ready)
}

#[tokio::test(flavor = "multi_thread")]
async fn installer_update_linux_authorizes_install_and_allows_retry() {
    use crate::app::{App, Message, message::ViewMessage};

    let (_root, ready) = fixture(InstallerKind::Rpm);
    let mut app = App::new().0;
    let _ = app.update(Message::View(ViewMessage::OpenAppUpdateModal));
    app.desktop_update.busy = true;
    let _ = app.update(Message::View(ViewMessage::AppUpdatePrepared(Ok(ready.clone()))));
    assert!(app.update_panel.cli_install_modal_message.contains("Ubuntu/Debian"));
    assert!(app.update_panel.cli_install_modal_message.contains("DEB"));

    let error = "无法启动自动更新：authorization unavailable";
    app.desktop_update.busy = true;
    let _ = app.update(Message::View(ViewMessage::AppUpdateFinished(Err(error.to_string()))));
    assert_eq!(app.update_panel.cli_install_modal_title, "自动更新启动失败");
    assert!(app.toast_state.active_toast.is_some());
    assert!(!app.desktop_update.busy);
    assert!(!app.shutdown_requested);
    assert!(app.desktop_update.ready.is_some());

    let message = format!("自动更新已准备就绪，正在退出应用；更新成功后将提示启动新版。{}", ready.instructions());
    app.desktop_update.busy = true;
    app.main_window_id = Some(iced::window::Id::unique());
    let _ = app.update(Message::View(ViewMessage::AppUpdateFinished(Ok(message))));
    assert!(app.shutdown_requested);
    assert!(!app.desktop_update.busy);
    assert!(app.desktop_update.ready.is_some());
    assert!(ready.path().exists());
}
#[tokio::test(flavor = "multi_thread")]
async fn installer_directory_failure_preserves_package_and_does_not_exit() {
    use crate::app::{App, Message, message::ViewMessage};
    use iced_runtime::futures::futures::StreamExt;

    let mut app = App::new().0;
    assert_eq!(app.update(Message::View(ViewMessage::OpenAppInstallerDirectory)).units(), 0);
    let (_root, ready) = fixture(InstallerKind::Dmg);
    app.desktop_update.ready = Some(ready.clone());
    app.desktop_update.busy = true;
    assert_eq!(app.update(Message::View(ViewMessage::OpenAppInstallerDirectory)).units(), 0);
    app.desktop_update.busy = false;
    std::fs::write(ready.path(), b"tampered").unwrap();
    let task = app.update(Message::View(ViewMessage::OpenAppInstallerDirectory));
    let mut stream = iced_runtime::task::into_stream(task).unwrap();
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), stream.next())
        .await
        .unwrap()
        .unwrap();
    match result {
        iced_runtime::Action::Output(
            message @ Message::View(ViewMessage::AppInstallerDirectoryOpened(Err(_))),
        ) => {
            let _ = app.update(message);
        }
        _ => panic!("tampered package must not reach the file manager"),
    }
    assert!(app.toast_state.active_toast.is_some());
    assert!(!app.shutdown_requested);
    assert!(!app.desktop_update.busy);
    assert_eq!(app.desktop_update.ready.as_ref().unwrap().path(), ready.path());
    let _ = app.update(Message::View(ViewMessage::AppInstallerDirectoryOpened(Ok(()))));
    assert!(!app.shutdown_requested);
    assert!(ready.path().exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn app_update_ready_installer_survives_close_open_failure_and_retry() {
    use crate::app::{App, Message, message::ViewMessage};
    use iced::advanced::{renderer::Headless, widget::Tree};
    use iced_runtime::futures::futures::StreamExt;

    let (_root, ready) = fixture(InstallerKind::Dmg);
    let mut app = App::new().0;
    let _ = app.update(Message::View(ViewMessage::OpenAppUpdateModal));
    app.desktop_update.busy = true;
    let _ = app.update(Message::View(ViewMessage::AppUpdatePrepared(Ok(ready.clone()))));
    assert_eq!(app.update_panel.cli_install_modal_title, "安装包已就绪");
    assert!(app.update_panel.cli_install_modal_message.contains(ready.path().to_str().unwrap()));
    let _ = app.update(Message::View(ViewMessage::CloseInstallCliModal));
    let _ = app.update(Message::View(ViewMessage::OpenAppUpdateModal));
    assert_eq!(app.desktop_update.ready.as_ref().unwrap().path(), ready.path());

    // Exercise the real async opener route without launching any installer:
    // a changed verified file must be rejected before reaching the OS opener.
    std::fs::write(ready.path(), b"tampered").unwrap();
    let task = app.update(Message::View(ViewMessage::OpenAppInstaller));
    assert!(app.desktop_update.busy);
    assert!(app.update_panel.cli_install_modal_is_checking_update);
    assert_eq!(app.update_panel.cli_install_modal_title, "更新中…");
    assert!(app.update_panel.cli_install_modal_message.contains("正在校验安装包"));
    let duplicate = app.update(Message::View(ViewMessage::OpenAppInstaller));
    assert_eq!(duplicate.units(), 0);
    let mut stream = iced_runtime::task::into_stream(task).unwrap();
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), stream.next())
        .await
        .unwrap()
        .unwrap();
    match result {
        iced_runtime::Action::Output(
            message @ Message::View(ViewMessage::AppUpdateFinished(Err(_))),
        ) => {
            let _ = app.update(message);
        }
        _ => panic!("tampered installer must fail before opening"),
    }
    assert!(!app.desktop_update.busy);
    assert!(!app.update_panel.cli_install_modal_is_checking_update);
    assert!(!app.shutdown_requested);
    assert_eq!(app.update_panel.cli_install_modal_title, "自动更新启动失败");
    assert!(app.toast_state.active_toast.is_some());
    assert!(app.update_panel.cli_install_modal_message.contains(ready.path().to_str().unwrap()));
    assert!(app.desktop_update.ready.is_some());

    // Retry the same retained file with an injected opener; never touch the system installer.
    std::fs::write(ready.path(), b"verified").unwrap();
    let message = format!("自动更新已准备就绪，正在退出应用；更新成功后将提示启动新版。{}", ready.instructions());
    app.desktop_update.busy = true;
    app.main_window_id = Some(iced::window::Id::unique());
    let _ = app.update(Message::View(ViewMessage::AppUpdateFinished(Ok(message))));
    assert!(app.shutdown_requested);
    assert_eq!(app.update_panel.cli_install_modal_title, "安装包已就绪");
    assert!(!app.update_panel.cli_install_modal_message.contains("更新完成"));
    assert!(ready.path().exists());

    // Draw every loading frame and the ready/retry controls in both themes.
    let mut renderer = <iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(16.0),
        Some("tiny-skia"),
    )
    .await
    .unwrap();
    for (kind, theme) in [
        (InstallerKind::Dmg, iced::Theme::Dark),
        (InstallerKind::Dmg, iced::Theme::Light),
        (InstallerKind::Rpm, iced::Theme::Dark),
        (InstallerKind::Rpm, iced::Theme::Light),
        (InstallerKind::Deb, iced::Theme::Dark),
        (InstallerKind::Deb, iced::Theme::Light),
    ] {
        let (_root, ready) = fixture(kind);
        app.update_panel.cli_install_modal_message = ready.instructions();
        app.desktop_update.ready = Some(ready);
        for frame in 0..=10 {
            app.desktop_update.busy = frame < 10;
            app.update_panel.cli_install_modal_is_checking_update = app.desktop_update.busy;
            app.status_animation_frame = frame;
            let mut element = crate::app::components::install_cli_modal::view_app_update(&app);
            let viewport = iced::Rectangle::with_size(iced::Size::new(960.0, 900.0));
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &iced::advanced::layout::Limits::new(iced::Size::ZERO, viewport.size()),
            );
            iced::advanced::Renderer::reset(&mut renderer, viewport);
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &iced::advanced::renderer::Style::default(),
                iced::advanced::Layout::new(&node),
                iced::mouse::Cursor::Unavailable,
                &viewport,
            );
            let pixels =
                renderer.screenshot(iced::Size::new(960, 900), 1.0, iced::Color::TRANSPARENT);
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
        }
    }
}
