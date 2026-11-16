#[test]
fn discard_file_modal_requires_target_file_and_builds_stack() {
    let mut file = None;

    assert!(
        crate::git_panel::panel::discard_file_modal(
            config_ui::DesktopLocale::default(),
            file.as_deref()
        )
        .is_none()
    );

    file = Some("src/lib.rs".to_string());
    let modal = crate::git_panel::panel::discard_file_modal(
        config_ui::DesktopLocale::default(),
        file.as_deref(),
    )
    .expect("discard modal should render");

    assert_eq!(modal.as_widget().children().len(), 2);
}
