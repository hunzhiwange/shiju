//! Host contract: each desktop entry reaches the domain modal update.
use super::*;
use crate::app::message::settings;
#[test]
fn wecom_delivery_entries_map_to_domain_modal() {
    let mut app = App::new().0;
    for mode in [crate::app::state::WeComDeliveryMode::Qr, crate::app::state::WeComDeliveryMode::Manual, crate::app::state::WeComDeliveryMode::Webhook] {
        drop(settings::update(&mut app, SettingsMessage::Channels(ChannelsMessage::WeComDeliveryOpened(mode))));
        assert_eq!(app.channels_settings.wecom_delivery.as_ref().unwrap().mode, mode);
    }
}
