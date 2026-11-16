//! Panel rendering tests use local borrowed inputs and the shared rendering fixture.
mod global_status_tests;
mod message_view_tests;
mod status_fixture;
fn should_use_user_message_editor(
    app: &crate::host_module_5_tests::fixture::RenderFixture,
    idx: usize,
    content: &str,
    enabled: bool,
) -> bool {
    crate::message_view::bubble::should_use_user_message_editor(
        &app.snapshot(),
        idx,
        content,
        enabled,
    )
}

mod header_fixture;
mod header_tests;
mod performance_tests;

mod selectable_text_tests;
mod text_support {
    pub use desktop_chat_tools::tool_text_support::tool_text_key;
    pub fn selected_chat_text_for_message(
        app: &crate::host_module_5_tests::fixture::RenderFixture,
        idx: usize,
    ) -> Option<String> {
        desktop_chat_tools::tool_text_support::selected_chat_text_for_message(
            &app.text_state(),
            idx,
        )
    }
}
mod layout_tests;

mod message_measure_tests;
fn apply_chat_message_measurements(
    app: &mut crate::host_module_5_tests::fixture::RenderFixture,
    measurements: Vec<crate::message_measure::ChatMessageMeasurement>,
) -> bool {
    let changed = crate::scroll_update::apply_targeted_message_measurements(
        &mut app.chat_virtualizer,
        &mut app.chat_message_measured_heights,
        app.chat.len(),
        None,
        measurements,
    );
    if changed {
        app.chat_ui().sync_chat_virtualizer_compat_fields();
    }
    changed
}
