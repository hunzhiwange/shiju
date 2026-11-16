//! Native editor rendering data.
use crate::components::editor::Editor;
use crate::preview::PreviewTab;
pub fn tab() -> PreviewTab {
    let content = format!(
        "fn 中文配置验收() {{\n\tlet color = \"#ff8800\";\n    let nested = ([1, 2, 3]);\n    // {}\n{}\n}}",
        "中文长行 Tab 与软换行 ".repeat(25),
        (0..65)
            .map(|i| format!("    let row_{i} = ({i}, \"#22aa88\");"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let path = "/tmp/editor-config-acceptance.rs".to_owned();
    let tab = PreviewTab {
        path: path.clone(),
        title: "editor-config-acceptance.rs".into(),
        content: content.clone(),
        is_dirty: false,
        truncated: false,
        read_only: false,
        auto_save_revision: 0,
        editor: Editor::new(&content, "rust"),
        scroll_id: iced::widget::Id::unique(),
        lsp_server_key: None,
        lsp_uri: None,
        lsp_language_id: None,
    };
    tab
}
