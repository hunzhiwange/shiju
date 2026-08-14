//! Slash palette overlay 组件测试。

use super::slash_palette::slash_palette_overlay;
use crate::app::App;
use crate::app::state::{SkillsCatalogItem, SkillsCatalogKind, SlashPaletteKind};
use iced::Length;

fn catalog_skill(id: &str, title: &str) -> SkillsCatalogItem {
    SkillsCatalogItem {
        id: id.to_string(),
        title: title.to_string(),
        description: String::new(),
        kind: SkillsCatalogKind::Personal,
        resource_count: 0,
        installed: true,
        enabled: true,
        source: "test".to_string(),
        source_path: None,
    }
}

fn overlay_list_len(app: &App) -> usize {
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
    let mut app = App::new().0;
    app.show_slash_palette = true;
    app.slash_palette_query = "zzz-no-match".to_string();

    let overlay = slash_palette_overlay(&app);
    assert_eq!(overlay.as_widget().size().width, Length::Fixed(360.0));
    assert_eq!(overlay_list_len(&app), 1);
}

#[test]
fn slash_palette_overlay_renders_section_headers_and_show_more() {
    let mut app = App::new().0;
    app.show_slash_palette = true;
    app.slash_palette_query = String::new();
    app.skills_settings.catalog = vec![
        catalog_skill("automate", "automate"),
        catalog_skill("babysit", "babysit"),
        catalog_skill("baidu-trending", "baidu-trending"),
        catalog_skill("code-review", "code-review"),
        catalog_skill("writing-plans", "writing-plans"),
    ];
    app.acp_agents = vec!["codex".to_string(), "claude".to_string(), "opencode".to_string()];

    let overlay = slash_palette_overlay(&app);
    assert_eq!(overlay.as_widget().size().width, Length::Fixed(360.0));
    drop(overlay);

    // Skills header + 3 entries + Show more + spacer, plus other sections.
    let len = overlay_list_len(&app);
    assert!(len > 5, "expected grouped rows with show-more, got {len}");

    // Expanding skills should remove the show-more row for that kind.
    app.slash_palette_expanded_kinds.insert(SlashPaletteKind::Skill);
    let expanded_len = overlay_list_len(&app);
    assert!(expanded_len >= len, "expanded list should keep or grow visible rows");
}
