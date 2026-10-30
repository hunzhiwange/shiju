use super::*;
use sha2::{Digest, Sha256};

fn asset(kind: InstallerKind) -> DesktopInstaller {
    DesktopInstaller {
        name: format!("installer.{}", extension(kind)),
        target: String::new(),
        kind,
        url: String::new(),
        size: 8,
        sha256: hex::encode(Sha256::digest(b"verified")),
    }
}
fn fixture(kind: InstallerKind) -> (tempfile::TempDir, PreparedDesktopUpdate) {
    let root = tempfile::tempdir().unwrap();
    let directory = create_attempt(root.path(), "1.2.3").unwrap();
    let asset = asset(kind);
    let path = directory.path().join(&asset.name);
    std::fs::write(&path, b"verified").unwrap();
    let ready = finish_download(directory, path, &asset, "1.2.3").unwrap();
    (root, ready)
}
#[test]
fn installer_update_six_platforms_use_expected_extensions() {
    for (os, kind) in [
        ("macos", InstallerKind::Dmg),
        ("windows", InstallerKind::Msi),
        ("linux", InstallerKind::Rpm),
    ] {
        for arch in ["x86_64", "aarch64"] {
            assert!(desktop_target(os, arch).unwrap().starts_with(arch));
            let (_root, ready) = fixture(kind);
            open_with(&ready, os, |path| {
                if kind == InstallerKind::Rpm {
                    assert_eq!(path, ready.path().parent().unwrap());
                    assert!(path.is_dir());
                } else {
                    assert_eq!(path, ready.path());
                    assert_eq!(path.extension().unwrap(), extension(kind));
                }
                assert!(path.is_absolute());
                Ok(())
            })
            .unwrap();
        }
    }
    assert!(desktop_target("linux", "arm").is_err());
}
#[test]
fn installer_update_unicode_cache_and_retries_preserve_files() {
    let root = tempfile::tempdir().unwrap();
    let cache = root.path().join("中文 cache");
    std::fs::create_dir(&cache).unwrap();
    let first = create_attempt(&cache, "1.2.3").unwrap();
    let second = create_attempt(&cache, "1.2.3").unwrap();
    assert_ne!(first.path(), second.path());
    assert!(first.path().starts_with(cache.canonicalize().unwrap().join("updates/v1.2.3")));
    let asset = asset(InstallerKind::Dmg);
    let path = first.path().join(&asset.name);
    std::fs::write(&path, b"verified").unwrap();
    let ready = finish_download(first, path.clone(), &asset, "1.2.3").unwrap();
    drop(second);
    let error = open_with(&ready, "macos", |_| {
        Err(anyhow::anyhow!("no handler").context("helper launch failed"))
    })
    .unwrap_err()
    .to_string();
    assert!(error.contains("helper launch failed: no handler"));
    assert!(!error.contains("安装包已就绪"));
    open_with(&ready, "macos", |_| Ok(())).unwrap();
    drop(ready);
    assert_eq!(std::fs::read(path).unwrap(), b"verified");
}
#[test]
fn installer_update_unverified_and_modified_files_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let directory = create_attempt(root.path(), "1.2.3").unwrap();
    let path = directory.path().join("installer.dmg");
    std::fs::write(&path, b"partial").unwrap();
    assert!(finish_download(directory, path.clone(), &asset(InstallerKind::Dmg), "1.2.3").is_err());
    assert!(!path.exists());
    let (_root, ready) = fixture(InstallerKind::Dmg);
    std::fs::write(ready.path(), b"tampered").unwrap();
    assert!(open_with(&ready, "macos", |_| panic!("must not open")).is_err());
}
#[test]
fn installer_update_missing_download_and_wrong_platform_never_open() {
    let root = tempfile::tempdir().unwrap();
    let directory = create_attempt(root.path(), "1.2.3").unwrap();
    let path = directory.path().join("installer.dmg");
    assert!(finish_download(directory, path, &asset(InstallerKind::Dmg), "1.2.3").is_err());
    let (_root, ready) = fixture(InstallerKind::Rpm);
    assert!(open_with(&ready, "windows", |_| panic!("must not open")).is_err());
    let message = open_with(&ready, "linux", |_| Ok(())).unwrap();
    assert!(message.contains("RPM"));
    assert!(!message.contains("升级完成"));
}

#[tokio::test(flavor = "multi_thread")]
async fn installer_update_linux_opens_directory_preserves_app_and_allows_retry() {
    use crate::app::{App, Message, message::ViewMessage};

    let (_root, ready) = fixture(InstallerKind::Rpm);
    assert!(!ready.installs_automatically());
    let mut app = App::new().0;
    let _ = app.update(Message::View(ViewMessage::OpenAppUpdateModal));
    app.desktop_update.busy = true;
    let _ = app.update(Message::View(ViewMessage::AppUpdatePrepared(Ok(ready.clone()))));
    assert!(app.cli_install_modal_message.contains("Ubuntu/Debian"));
    assert!(app.cli_install_modal_message.contains(".deb"));

    let error = open_with(&ready, "linux", |path| {
        assert_eq!(path, ready.path().parent().unwrap());
        Err(anyhow::anyhow!("file manager unavailable"))
    })
    .unwrap_err();
    assert!(error.to_string().contains("file manager unavailable"));
    app.desktop_update.busy = true;
    let _ = app.update(Message::View(ViewMessage::AppUpdateFinished(Err(error.to_string()))));
    assert_eq!(app.cli_install_modal_title, "打开下载位置失败");
    assert!(app.active_toast.is_some());
    assert!(!app.desktop_update.busy);
    assert!(!app.shutdown_requested);
    assert!(app.desktop_update.ready.is_some());

    let message = open_with(&ready, "linux", |path| {
        assert!(path.is_dir());
        assert_eq!(path, ready.path().parent().unwrap());
        Ok(())
    })
    .unwrap();
    assert!(message.contains("下载位置已打开"));
    assert!(!message.contains("正在退出"));
    app.desktop_update.busy = true;
    app.main_window_id = Some(iced::window::Id::unique());
    let _ = app.update(Message::View(ViewMessage::AppUpdateFinished(Ok(message))));
    assert!(!app.shutdown_requested);
    assert!(!app.desktop_update.busy);
    assert!(app.desktop_update.ready.is_some());
    assert!(ready.path().exists());

    // Recheck the RPM before revealing its directory, just as for automatic installers.
    std::fs::write(ready.path(), b"tampered").unwrap();
    assert!(open_with(&ready, "linux", |_| panic!("must not open")).is_err());
}
#[cfg(unix)]
#[test]
fn installer_update_symlink_escape_is_rejected() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    symlink(outside.path(), root.path().join("updates")).unwrap();
    assert!(create_attempt(root.path(), "1.2.3").is_err());
    let (_root, ready) = fixture(InstallerKind::Dmg);
    let external = outside.path().join("installer.dmg");
    std::fs::write(&external, b"verified").unwrap();
    std::fs::remove_file(ready.path()).unwrap();
    symlink(&external, ready.path()).unwrap();
    assert!(open_with(&ready, "macos", |_| panic!("must not open")).is_err());
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
    assert_eq!(app.cli_install_modal_title, "安装包已就绪");
    assert!(app.cli_install_modal_message.contains(ready.path().to_str().unwrap()));
    let _ = app.update(Message::View(ViewMessage::CloseInstallCliModal));
    let _ = app.update(Message::View(ViewMessage::OpenAppUpdateModal));
    assert_eq!(app.desktop_update.ready.as_ref().unwrap().path(), ready.path());

    // Exercise the real async opener route without launching any installer:
    // a changed verified file must be rejected before reaching the OS opener.
    std::fs::write(ready.path(), b"tampered").unwrap();
    let task = app.update(Message::View(ViewMessage::OpenAppInstaller));
    assert!(app.desktop_update.busy);
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
    assert!(!app.shutdown_requested);
    assert_eq!(app.cli_install_modal_title, "自动更新启动失败");
    assert!(app.active_toast.is_some());
    assert!(app.cli_install_modal_message.contains(ready.path().to_str().unwrap()));
    assert!(app.desktop_update.ready.is_some());

    // Retry the same retained file with an injected opener; never touch the system installer.
    std::fs::write(ready.path(), b"verified").unwrap();
    let message = open_with(&ready, "macos", |path| {
        assert_eq!(path, ready.path());
        Ok(())
    })
    .unwrap();
    app.desktop_update.busy = true;
    app.main_window_id = Some(iced::window::Id::unique());
    let _ = app.update(Message::View(ViewMessage::AppUpdateFinished(Ok(message))));
    assert!(app.shutdown_requested);
    assert_eq!(app.cli_install_modal_title, "安装包已就绪");
    assert!(!app.cli_install_modal_message.contains("更新完成"));
    assert!(ready.path().exists());

    // Draw the real ready-state controls, including the open/retry action, in both themes.
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
    ] {
        let (_root, ready) = fixture(kind);
        app.cli_install_modal_message = ready.instructions();
        app.desktop_update.ready = Some(ready);
        let mut element = crate::app::components::install_cli_modal::view_app_update(&app);
        let viewport = iced::Rectangle::with_size(iced::Size::new(960.0, 900.0));
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &iced::advanced::layout::Limits::new(iced::Size::ZERO, viewport.size()),
        );
        element.as_widget().draw(
            &tree,
            &mut renderer,
            &theme,
            &iced::advanced::renderer::Style::default(),
            iced::advanced::Layout::new(&node),
            iced::mouse::Cursor::Unavailable,
            &viewport,
        );
        let pixels = renderer.screenshot(iced::Size::new(960, 900), 1.0, iced::Color::TRANSPARENT);
        assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
    }
}
