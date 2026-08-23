//! 会话信息的前端字段契约与计划运行标记测试。
//!
//! 覆盖可选字段省略、嵌套回退信息以及 slug 标记的幂等往返。

#[test]
fn info_serialization_uses_frontend_field_names_and_skips_empty_options() {
    let info = super::Info {
        id: "session-1".to_string(),
        slug: "slug".to_string(),
        project_id: "project-1".to_string(),
        directory: "/work".to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: "Title".to_string(),
        version: "1".to_string(),
        time: super::TimeInfo { created: 1, updated: 2, compacting: None, archived: None },
        permission: None,
        revert: Some(super::RevertInfo {
            message_id: "msg-1".to_string(),
            part_id: None,
            snapshot: None,
            diff: None,
        }),
    };

    let value = serde_json::to_value(info).unwrap();

    assert_eq!(value["projectID"], "project-1");
    assert_eq!(value["revert"]["messageID"], "msg-1");
    assert!(value.get("parentID").is_none());
    assert!(value["revert"].get("partID").is_none());
}

#[test]
fn plan_run_session_marker_round_trips() {
    let ordinary = super::Info {
        id: "session-1".to_string(),
        slug: "ordinary-slug".to_string(),
        project_id: "project-1".to_string(),
        directory: "/work".to_string(),
        parent_id: Some("parent-1".to_string()),
        summary: None,
        share: None,
        title: "Plan run".to_string(),
        version: "1".to_string(),
        time: super::TimeInfo { created: 1, updated: 2, compacting: None, archived: None },
        permission: None,
        revert: None,
    };
    assert!(!ordinary.is_plan_run());

    let mut plan_run = ordinary.clone();
    plan_run.slug = super::Info::plan_run_slug(&plan_run.slug);
    assert!(plan_run.is_plan_run());
    assert_eq!(super::Info::plan_run_slug(&plan_run.slug), plan_run.slug);

    let value = serde_json::to_value(&plan_run).unwrap();
    assert!(value.get("isPlanRun").is_none());

    let restored: super::Info = serde_json::from_value(value).unwrap();
    assert!(restored.is_plan_run());
    assert_eq!(restored.slug, plan_run.slug);
}
