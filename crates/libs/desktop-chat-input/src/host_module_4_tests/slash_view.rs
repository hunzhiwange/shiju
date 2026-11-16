use crate::input_panel::{slash_palette, suggestions::State};
use desktop_session::message::chat::input::slash_palette::{
    SlashPaletteCandidate, SlashPaletteCandidates, SlashPaletteKind, build_ranked_slash_sections,
};
use iced::Length;
#[derive(Default)]
struct Fixture {
    state: State,
    candidates: SlashPaletteCandidates,
}
fn catalog_skill(id: &str, title: &str) -> SlashPaletteCandidate {
    SlashPaletteCandidate { id: id.into(), label: title.into(), description: None }
}
fn slash_palette_overlay(app: &Fixture) -> iced::Element<'static, slash_palette::Message> {
    let sections = build_ranked_slash_sections(
        &app.state.slash_palette_query,
        &app.candidates,
        &app.state.slash_palette_expanded_kinds,
    );
    slash_palette::slash_palette_overlay(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        &sections,
        &app.state.slash_palette_query,
        app.state.slash_palette_selected_index,
        iced::widget::Id::new("slash-fixture"),
    )
}
fn overlay_list_len(app: &Fixture) -> usize {
    let overlay = slash_palette_overlay(app);
    let root_children = overlay.as_widget().children();
    root_children
        .get(1)
        .and_then(|scrollable| scrollable.children.first())
        .expect("overlay should contain the result list")
        .children
        .len()
}

#[test]
fn slash_palette_overlay_renders_empty_state() {
    let mut app = Fixture::default();
    app.state.show_slash_palette = true;
    app.state.slash_palette_query = "zzz-no-match".to_string();

    let overlay = slash_palette_overlay(&app);
    assert_eq!(overlay.as_widget().size().width, Length::Fixed(360.0));
    assert_eq!(overlay_list_len(&app), 1);
}

#[test]
fn slash_palette_overlay_renders_section_headers_and_show_more() {
    let mut app = Fixture::default();
    app.state.show_slash_palette = true;
    app.state.slash_palette_query = String::new();
    app.candidates.skills = vec![
        catalog_skill("automate", "automate"),
        catalog_skill("babysit", "babysit"),
        catalog_skill("baidu-trending", "baidu-trending"),
        catalog_skill("code-review", "code-review"),
        catalog_skill("writing-plans", "writing-plans"),
    ];

    let overlay = slash_palette_overlay(&app);
    assert_eq!(overlay.as_widget().size().width, Length::Fixed(360.0));
    drop(overlay);

    // Skills header + 3 entries + Show more + spacer, plus other sections.
    let len = overlay_list_len(&app);
    assert!(len > 5, "expected grouped rows with show-more, got {len}");

    // Expanding skills should remove the show-more row for that kind.
    app.state.slash_palette_expanded_kinds.insert(SlashPaletteKind::Skill);
    let expanded_len = overlay_list_len(&app);
    assert!(expanded_len >= len, "expanded list should keep or grow visible rows");
}
