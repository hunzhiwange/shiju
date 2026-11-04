use crate::workflow::templates::VideoEditTemplateSeed;

#[test]
fn video_edit_entry_rejects_uncontrolled_graph_before_storage() {
    let yaml = r#"workflow:
  graph:
    nodes:
      - id: start
        data: {type: start}
      - id: unsafe
        data: {type: tool, provider: shiju, tool_name: bash, inputs: {command: 'echo ready'}}
    edges:
      - {source: start, target: unsafe}
"#;
    assert!(
        VideoEditTemplateSeed::new(yaml.into()).is_err(),
        "a tenant video template must reject arbitrary native bash"
    );
}

#[test]
fn video_edit_entry_distributes_production_graph() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("resources/workflows/cloud_video_edit.yaml");
    assert!(path.is_file(), "production graph must be distributed outside testdata");
}

#[test]
fn video_edit_entry_preserves_menu_and_unrelated_commands() {
    let menu = "剪辑 \"a.mp4\"\n\n<session_control_selection>\n技能：other, shiju\n</session_control_selection>";
    assert_eq!(super::entry::command_query(menu), "/shiju 剪辑 \"a.mp4\"");
    for query in ["/shiju 登录", "/shiju 剪辑器"] {
        assert!(
            super::parse_video_edit_command(&super::entry::command_query(query)).unwrap().is_none()
        );
    }
    assert!(super::parse_video_edit_command("/shiju 剪辑").is_err());
    assert!(super::parse_video_edit_command("/shiju 剪辑 a.mp4 b.mp4").is_err());
}

#[test]
fn video_edit_entry_rejects_capability_order_and_fake_ready_edits() {
    let yaml = super::DEFAULT_VIDEO_EDIT_YAML;
    for edit in [
        yaml.replace("provider: shiju_video", "provider: shiju"),
        yaml.replace("tool_name: apply", "tool_name: bash"),
        yaml.replace("type: llm", "type: agent"),
        yaml.replace("type: llm", "type: http-request"),
        yaml.replace("type: llm", "type: code"),
        yaml.replace("max_parallel: 5", "max_parallel: 6"),
        yaml.replace("target: preview", "target: answer"),
        yaml.replace("{{#preview.text#}}", "ready"),
        yaml.replace("value_selector: [planning, output]", "value: []"),
    ] {
        assert!(VideoEditTemplateSeed::new(edit).is_err());
    }
}

#[test]
fn video_edit_entry_accepts_business_prompt_model_and_lower_budget() {
    let mut value: serde_json::Value =
        serde_yaml::from_str(super::DEFAULT_VIDEO_EDIT_YAML).unwrap();
    value["workflow"]["graph"]["nodes"][0]["position"] = serde_json::json!({"x":20,"y":30});
    value["workflow"]["graph"]["nodes"][0]["data"]["title"] = serde_json::json!("编辑标题");
    value["workflow"]["video_edit"]["max_parallel"] = serde_json::json!(2);
    value["workflow"]["graph"]["nodes"][4]["data"]["max_parallel"] = serde_json::json!(2);
    value["workflow"]["graph"]["nodes"][3]["data"]["model"] =
        serde_json::json!({"name":"explicit/model"});
    value["workflow"]["graph"]["nodes"][3]["data"]["prompt_template"][0]["text"] =
        serde_json::json!("edited business brief");
    VideoEditTemplateSeed::new(serde_json::to_string(&value).unwrap()).unwrap();
}
