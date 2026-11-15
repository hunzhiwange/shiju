#![cfg(not(target_arch = "wasm32"))]
use crate::app::message::settings::worktrees::{WorktreeSettingsMessage as Event, apply};
use crate::app::state::{WorktreeSettingsIdentity as Identity, WorktreeSettingsState as State};
use config_worktree::WorktreeSettings;

fn identity() -> Identity {
    Identity {
        gateway: "http://fixture".into(),
        tenant: Default::default(),
        auth_fingerprint: [0; 32],
        project: Some("/project".into()),
    }
}
fn load(state: &mut State, current: &Identity, policy: WorktreeSettings) {
    apply(state, Event::Refresh, current);
    let generation = state.generation;
    apply(
        state,
        Event::Loaded {
            identity: current.clone(),
            generation,
            result: Ok((policy, "/server/default".into())),
        },
        current,
    );
}
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn worktree_settings_ui_renders_chinese_modal_in_both_themes() {
    use super::system_settings_worktrees::{editor, page};
    use crate::app::i18n::I18n;
    use iced::advanced::{
        clipboard,
        renderer::{Headless, Style},
    };
    use iced::{Font, Renderer, Size, Theme, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};
    use config_ui::DesktopLocale;
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
            .await
            .unwrap();
    let (mut state, id) = (State::default(), identity());
    load(&mut state, &id, WorktreeSettings::default());
    apply(&mut state, Event::Edit, &id);
    apply(&mut state, Event::RootChanged("/项目/工作树".into()), &id);
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        for theme in [Theme::Light, Theme::Dark] {
            let i18n = I18n::new(locale);
            let size = Size::new(860.0, 700.0);
            let mut ui = UserInterface::build(
                editor(&state, i18n, page(&state, i18n)),
                size,
                Cache::new(),
                &mut renderer,
            );
            ui.update(
                &[],
                mouse::Cursor::Unavailable,
                &mut renderer,
                &mut clipboard::Null,
                &mut Vec::new(),
            );
            ui.draw(
                &mut renderer,
                &theme,
                &Style { text_color: theme.palette().text },
                mouse::Cursor::Unavailable,
            );
            let pixels = renderer.screenshot(Size::new(860, 700), 1.0, theme.palette().background);
            assert_eq!(pixels.len(), 860 * 700 * 4);
            if let Some(root) = std::env::var_os("SHIJU_WORKTREE_SETTINGS_SCREENSHOTS") {
                std::fs::create_dir_all(&root).unwrap();
                image::save_buffer(
                    std::path::PathBuf::from(root).join(format!("{locale:?}-{theme:?}.png")),
                    &pixels,
                    860,
                    700,
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
        }
    }
}
