//! Rendering cases for the native voice acceptance entry.
use config_voice::VoiceConfig;
use desktop_voice::preferences::{Edit, Preferences};
pub fn setup(state: &mut crate::State, case: usize) {
    state.edit = None;
    if case % 3 != 0 {
        state.resources = crate::local_resources(None);
        state.edit = Some(Edit::new(
            VoiceConfig {
                dictionary: (0..45).map(|n| format!("中文词典条目{n}")).collect(),
                ..Default::default()
            },
            Preferences::default(),
        ));
    }
}
