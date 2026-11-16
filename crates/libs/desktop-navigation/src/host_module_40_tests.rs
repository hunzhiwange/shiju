//! Uses the rendering fixture called by the native tray entry.
#[test]
fn tray_fixture_builds() {
    let _: iced::Element<'static, ()> = crate::acceptance::tray_view();
}
