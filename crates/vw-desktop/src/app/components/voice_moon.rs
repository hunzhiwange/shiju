//! Blue-white local voice orb, rendered through the existing SVG backend.
use crate::app::Message;
use iced::{Element, widget::svg};

pub(super) fn view(muted: bool) -> Element<'static, Message> {
    let colors = if muted { ("#748399", "#c2ccd9") } else { ("#3066eb", "#8fb6ff") };
    let data = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="96" height="96" viewBox="0 0 96 96"><defs><linearGradient id="moon" x1="0.2" y1="0.1" x2="0.7" y2="1"><stop offset="0" stop-color="{}"/><stop offset="0.48" stop-color="{}"/><stop offset="0.7" stop-color="#eef5ff"/><stop offset="1" stop-color="#b8ceff"/></linearGradient></defs><circle cx="48" cy="48" r="47" fill="url(#moon)"/></svg>"##,
        colors.0, colors.1
    );
    svg(svg::Handle::from_memory(data.into_bytes())).width(96).height(96).into()
}
