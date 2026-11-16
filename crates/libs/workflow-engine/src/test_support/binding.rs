use serde_json::json;
pub fn binding_fixture(tenant: vw_api_transport::TenantScope) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(".shiju")).unwrap();
    std::fs::write(
        dir.path().join(".shiju/workflow_commands.json"),
        json!({
            "platform_id":tenant.platform_id().as_ref(),
            "company_id":tenant.company_id().as_ref(),
            "commands":[{"command":"/shiju 剪辑","workflow_file":"preview.yaml","inputs":{}}]
        })
        .to_string(),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("preview.yaml"),
        r#"
workflow:
  graph:
    nodes:
      - id: start
        data: {type: start}
      - id: answer
        data: {type: answer, answer: 'verified preview'}
    edges:
      - {source: start, target: answer}
"#,
    )
    .unwrap();
    dir
}
