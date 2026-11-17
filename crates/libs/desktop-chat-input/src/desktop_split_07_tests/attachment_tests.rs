use crate::input_panel::attachment::{Message, attachment_button, attachment_strip};
fn i18n() -> desktop_i18n::I18n {
    desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese)
}
use iced::{Element, Length};
fn assert_size(element: &Element<'_, Message>, width: Length, height: Length) {
    let size = element.as_widget().size();

    assert_eq!(size.width, width);
    assert_eq!(size.height, height);
}

#[test]
fn attachment_button_and_strip_reflect_empty_and_populated_app_files() {
    let mut files = Vec::new();
    let max_images = 1;
    let max_size = 1;

    let button = attachment_button(
        i18n(),
        &files,
        max_images,
        max_size,
        crate::input_panel::styles::square_icon_button_style,
    );
    assert_size(&button, Length::Shrink, Length::Shrink);
    drop(button);

    let empty_strip = attachment_strip(i18n(), &files);
    assert_size(&empty_strip, Length::Shrink, Length::Fixed(0.0));
    drop(empty_strip);

    files = vec!["/tmp/report.pdf".to_string(), "/tmp/screenshot.PNG".to_string()];
    let populated_strip = attachment_strip(i18n(), &files);
    assert_size(&populated_strip, Length::Shrink, Length::Shrink);
}

#[test]
fn attachment_button_clamps_multimodal_limits_while_counting_images() {
    let files =
        vec!["/tmp/one.png".to_string(), "/tmp/two.JPG".to_string(), "/tmp/readme.md".to_string()];
    let mut max_images = 0;
    let mut max_size = 99;

    let lower_clamped = attachment_button(
        i18n(),
        &files,
        max_images,
        max_size,
        crate::input_panel::styles::square_icon_button_style,
    );
    assert_size(&lower_clamped, Length::Shrink, Length::Shrink);
    drop(lower_clamped);

    max_images = 99;
    max_size = 0;
    let upper_clamped = attachment_button(
        i18n(),
        &files,
        max_images,
        max_size,
        crate::input_panel::styles::square_icon_button_style,
    );
    assert_size(&upper_clamped, Length::Shrink, Length::Shrink);
}
