use super::support::*;
fn app() -> Fixture {
    Fixture::default()
}
fn apply(app: &mut Fixture, message: MemoryMessage) {
    let _ = update(app, SettingsMessage::Memory(message));
}
#[test]
fn tab_selection_updates_only_visible_memory_section() {
    let mut app = app();
    app.memory_settings.save_error = Some("keep".to_string());

    apply(&mut app, MemoryMessage::TabSelected(MemorySettingsTab::Embedding));
    assert_eq!(app.memory_settings.active_tab, MemorySettingsTab::Embedding);
    assert_eq!(app.memory_settings.save_error.as_deref(), Some("keep"));

    apply(&mut app, MemoryMessage::TabSelected(MemorySettingsTab::EmbeddingRoutes));
    assert_eq!(app.memory_settings.active_tab, MemorySettingsTab::EmbeddingRoutes);

    apply(&mut app, MemoryMessage::TabSelected(MemorySettingsTab::Storage));
    assert_eq!(app.memory_settings.active_tab, MemorySettingsTab::Storage);

    assert_eq!(
        update(
            &mut app,
            SettingsMessage::Memory(MemoryMessage::TabSelected(MemorySettingsTab::EmbeddingRoutes))
        ),
        crate::memory_update::Effect::ClearEmbeddingRouteFeedback
    );
    apply(&mut app, MemoryMessage::TabSelected(MemorySettingsTab::Qdrant));
    assert_eq!(app.memory_settings.active_tab, MemorySettingsTab::EmbeddingRoutes);

    apply(&mut app, MemoryMessage::BackendChanged("qdrant".to_string()));
    apply(&mut app, MemoryMessage::TabSelected(MemorySettingsTab::Qdrant));
    assert_eq!(app.memory_settings.active_tab, MemorySettingsTab::Qdrant);

    apply(&mut app, MemoryMessage::BackendChanged("sqlite".to_string()));
    assert_eq!(app.memory_settings.active_tab, MemorySettingsTab::General);
}

#[test]
fn refresh_clears_only_the_error() {
    let mut app = app();
    app.memory_settings.backend = "postgres".into();
    app.memory_settings.save_error = Some("old".into());
    assert_eq!(
        update(&mut app, SettingsMessage::Memory(MemoryMessage::Refresh)),
        crate::memory_update::Effect::None
    );
    assert_eq!(app.memory_settings.backend, "postgres");
    assert!(app.memory_settings.save_error.is_none());
}
