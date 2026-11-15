use super::*;
use desktop_voice::preferences::Preferences;
use config_voice::VoiceConfig;
#[cfg(target_os = "macos")]
#[test]
fn voice_task_13_route_change_cancels_activity_without_persisting() {
    use crate::app::voice::preferences::VoiceEngine;
    let (mut app, _) = App::new();
    app.show_system_settings = true;
    app.system_settings_tab = crate::app::components::system_settings::SystemTab::Voice;
    let _ = update(&mut app, Event::Open);
    let _ = update(&mut app, Event::Loaded(1, Ok(Loaded {
        config: VoiceConfig::default(), local: Preferences::default(), devices: vec![],
        warning: None, resources: LocalResources::default(), cloud_ready: false, cloud_error: None,
    })));
    app.voice.draft.phase = crate::app::message::voice_draft::Phase::Transcribing;
    let old = 1;
    let _ = update(&mut app, Event::Engine(VoiceEngine::Local));
    assert_ne!(app.voice.draft.phase, crate::app::message::voice_draft::Phase::Transcribing);
    let _ = update(&mut app, Event::LocalLanguage(Some("zh-CN".into())));
    let _ = update(&mut app, Event::LocalVoice(Some("com.apple.voice.fixture".into())));
    let _ = update(&mut app, Event::Cancel);
    let _ = update(&mut app, Event::CloudLoaded(old, Ok(VoiceConfig::default())));
    assert!(app.voice.settings.edit.is_none(), "late cloud load must not reopen cancelled edit");
}
