#[cfg(not(target_arch = "wasm32"))]
pub fn show_error_dialog(title: &str, description: &str) {
    #[cfg(target_os = "windows")]
    {
        let _ = rfd::MessageDialog::new()
            .set_title(title)
            .set_description(description)
            .set_level(rfd::MessageLevel::Error)
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
    }
    #[cfg(not(target_os = "windows"))]
    {
        eprintln!("{title}: {description}");
    }
}

#[cfg(test)]
#[path = "dialog_tests.rs"]
mod tests;
