use super::{RingState, ring_state};
use crate::app::{App, desktop_update::SkippedRelease, message::view::ViewMessage};
use desktop_release::{DesktopRelease, UpdateSource};
use update_types::UpdateProgress;

#[test]
fn update_ring_shows_only_a_new_release_from_the_selected_source() {
    let (mut app, _) = App::new();
    app.desktop_update.source = Some(UpdateSource::Shiju);
    app.desktop_update.release = None;
    assert!(ring_state(&app).is_none());

    app.desktop_update.release = Some(DesktopRelease {
        source: UpdateSource::Shiju,
        version: update_types::current_version().into(),
        assets: Vec::new(),
    });
    assert!(ring_state(&app).is_none());

    let version = "9999.0.0";
    app.desktop_update.release.as_mut().unwrap().version = version.into();
    assert_eq!(ring_state(&app), Some((version, RingState::Available)));

    app.desktop_update.skipped = Some(SkippedRelease {
        source: UpdateSource::Shiju,
        version: version.into(),
    });
    assert!(ring_state(&app).is_none());

    app.desktop_update.skipped = None;
    app.desktop_update.source = Some(UpdateSource::Github);
    assert!(ring_state(&app).is_none());
}

#[test]
fn update_ring_measures_download_bytes_and_waits_for_verification() {
    let mut progress = UpdateProgress {
        asset_name: "installer".into(),
        download_url: String::new(),
        downloaded_bytes: 512,
        total_bytes: None,
        bytes_per_second: 0.0,
        average_bytes_per_second: 0.0,
    };
    assert_eq!(RingState::downloading(None, false), RingState::Downloading(None));
    assert_eq!(RingState::downloading(Some(&progress), false), RingState::Downloading(None));

    progress.total_bytes = Some(1024);
    let downloading = RingState::downloading(Some(&progress), false);
    assert_eq!(downloading, RingState::Downloading(Some(50)));
    assert_eq!(downloading.percent(), 50);
    assert!(matches!(downloading.action(), Some(ViewMessage::OpenAppUpdateModal)));

    let paused = RingState::downloading(Some(&progress), true);
    assert_eq!(paused, RingState::Paused(Some(50)));
    assert_eq!(paused.percent(), 50);
    assert!(matches!(paused.action(), Some(ViewMessage::OpenAppUpdateModal)));

    progress.downloaded_bytes = 1024;
    let verifying = RingState::downloading(Some(&progress), false);
    assert_eq!(verifying, RingState::Verifying);
    assert_eq!(verifying.percent(), 100);
    assert!(matches!(verifying.action(), Some(ViewMessage::OpenAppUpdateModal)));
}

#[test]
fn update_ring_routes_clicks_to_the_shared_download_and_install_handlers() {
    assert!(matches!(RingState::Available.action(), Some(ViewMessage::RunAppUpdate)));
    assert!(matches!(RingState::Ready.action(), Some(ViewMessage::OpenAppInstaller)));
    assert!(RingState::Installing.action().is_none());
}
