//! 文件引用编辑入口迁移自宿主输入控制器。
use crate::input_panel::file_references::remove_file_reference;
use iced::widget::text_editor;

#[test]
fn remove_file_reference_deletes_mention_and_compacts_whitespace() {
    let mut editor = text_editor::Content::with_text("read @src/main.rs and @tests/input.rs");

    remove_file_reference(&mut editor, "src/main.rs");

    assert_eq!(editor.text(), "read and @tests/input.rs");
}

#[test]
fn remove_file_reference_removes_all_matching_mentions() {
    let mut editor = text_editor::Content::with_text("@src/main.rs compare @src/main.rs");

    remove_file_reference(&mut editor, "src/main.rs");

    assert_eq!(editor.text(), "compare");
}

#[test]
fn removing_chinese_file_keeps_other_quoted_path_whitespace() {
    let mut editor = text_editor::Content::with_text(
        r#"@原始视频.mov @"目录/两个  空格.txt" @原始视频.mov.bak"#,
    );
    remove_file_reference(&mut editor, "原始视频.mov");
    assert_eq!(editor.text(), r#"@"目录/两个  空格.txt" @原始视频.mov.bak"#);
}

mod file_search_view;
mod suggestions;
mod model_render;
#[path = "host_module_4_tests/editor_actions.rs"]
mod editor_actions_tests;
mod task_mode_tests;
mod agent_row;
#[cfg(not(target_arch = "wasm32"))]
mod send_controls;
mod slash_view;
mod configuration_layout;
#[cfg(not(target_arch = "wasm32"))]
mod voice_controls;
mod controls_layout;
mod acp_render;
#[cfg(not(target_arch = "wasm32"))]
mod recommendations;
mod input_menu;
mod tool_detail_tests;

#[test]
fn skill_title_uses_catalog_title_and_falls_back_to_id() {
    let titles = [("review", "代码审查")];
    assert_eq!(crate::input_panel::panel::skill_title("review", titles.into_iter()), "代码审查");
    assert_eq!(crate::input_panel::panel::skill_title("missing", titles.into_iter()), "missing");
}
mod ui_driver;
