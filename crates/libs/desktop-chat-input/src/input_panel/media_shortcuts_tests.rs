use super::{AttachmentRule, MediaMode, MediaModeInput, Shortcut};
use std::path::Path;

#[test]
fn media_shortcuts_project_all_five_modes() {
    let modes = Shortcut::ALL
        .into_iter()
        .filter_map(|shortcut| match shortcut {
            Shortcut::Media(mode) => Some(mode),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(Shortcut::ALL.len(), 7);
    assert_eq!(
        modes,
        [
            MediaMode::Image,
            MediaMode::Video,
            MediaMode::Speech,
            MediaMode::Vision,
            MediaMode::Transcription,
        ]
    );
    for mode in modes {
        assert_ne!(mode.placeholder(), "随便问点什么...");
    }
}

#[test]
fn media_shortcuts_apply_attachment_requirements() {
    for (mode, rule, accepted, rejected) in [
        (MediaMode::Image, AttachmentRule::Optional, Some("reference.PNG"), "clip.mp4"),
        (MediaMode::Video, AttachmentRule::Optional, Some("frame.jpg"), "clip.mp4"),
        (MediaMode::Speech, AttachmentRule::None, None, "voice.wav"),
        (MediaMode::Vision, AttachmentRule::Required, Some("clip.MP4"), "voice.wav"),
        (MediaMode::Transcription, AttachmentRule::Required, Some("voice.WAV"), "photo.png"),
    ] {
        assert_eq!(mode.attachment_rule(), rule);
        assert!(!mode.accepts(Path::new(rejected)));
        assert!(!mode.accepts(Path::new("no_extension")));
        if let Some(path) = accepted {
            assert!(mode.accepts(Path::new(path)));
        }
    }
    assert!(MediaMode::Vision.accepts(Path::new("photo.webp")));
    assert!(MediaMode::Transcription.accepts(Path::new("clip.mp4")));
    assert!(!MediaMode::Transcription.accepts(Path::new("clip.mov")));
    assert!(!MediaMode::Transcription.accepts(Path::new("voice.aac")));
}
