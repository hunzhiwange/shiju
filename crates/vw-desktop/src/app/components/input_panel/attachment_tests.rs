use super::{attachment_button, attachment_strip};
use crate::app::{App, Message};
use iced::{Element, Length};
fn assert_size(element: &Element<'_, Message>, width: Length, height: Length) {
    let size = element.as_widget().size();

    assert_eq!(size.width, width);
    assert_eq!(size.height, height);
}

#[test]
fn attachment_button_and_strip_reflect_empty_and_populated_app_files() {
    let mut app = App::new().0;

    let button = attachment_button(&app);
    assert_size(&button, Length::Shrink, Length::Shrink);
    drop(button);

    let empty_strip = attachment_strip(&app);
    assert_size(&empty_strip, Length::Shrink, Length::Fixed(0.0));
    drop(empty_strip);

    app.files = vec!["/tmp/report.pdf".to_string(), "/tmp/screenshot.PNG".to_string()];
    let populated_strip = attachment_strip(&app);
    assert_size(&populated_strip, Length::Shrink, Length::Shrink);
}


#[test]
fn attachment_button_clamps_multimodal_limits_while_counting_images() {
    let mut app = App::new().0;
    app.files =
        vec!["/tmp/one.png".to_string(), "/tmp/two.JPG".to_string(), "/tmp/readme.md".to_string()];
    app.multimodal_settings.max_images = 0;
    app.multimodal_settings.max_image_size_mb = 99;

    let lower_clamped = attachment_button(&app);
    assert_size(&lower_clamped, Length::Shrink, Length::Shrink);
    drop(lower_clamped);

    app.multimodal_settings.max_images = 99;
    app.multimodal_settings.max_image_size_mb = 0;
    let upper_clamped = attachment_button(&app);
    assert_size(&upper_clamped, Length::Shrink, Length::Shrink);
}
