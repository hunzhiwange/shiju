//! Tests drive the production update reducer and inspect its explicit host effects.
use crate::app::desktop_update::DesktopUpdate;
use crate::panel::{Effect, PanelMessage as ViewMessage, PanelState};
struct Fixture {
    desktop_update: DesktopUpdate,
    update_panel: PanelState,
    effects: Vec<Effect>,
}
impl Fixture {
    fn new() -> Self {
        Self {
            desktop_update: DesktopUpdate::from_config(&serde_json::json!({})),
            update_panel: PanelState::default(),
            effects: Vec::new(),
        }
    }
    fn update(&mut self, event: ViewMessage) -> iced::Task<ViewMessage> {
        crate::panel::update(
            &mut self.update_panel,
            &mut self.desktop_update,
            &mut self.effects,
            "0.6.0",
            #[cfg(not(target_arch = "wasm32"))]
            config_ui::DesktopLocale::SimplifiedChinese,
            event,
        )
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn error_visible(&self) -> bool {
        self.effects.iter().any(|effect| matches!(effect, Effect::Error(_)))
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn shutdown_requested(&self) -> bool {
        self.effects.iter().any(|effect| matches!(effect, Effect::Shutdown))
    }
}
#[cfg(not(target_arch = "wasm32"))]
mod cancel;
#[cfg(not(target_arch = "wasm32"))]
mod installer;
#[cfg(not(target_arch = "wasm32"))]
mod progress;
mod release;
