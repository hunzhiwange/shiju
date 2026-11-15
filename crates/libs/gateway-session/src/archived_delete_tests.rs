use super::delete_order;
use session_info::{Info, TimeInfo};

fn info(id: &str, parent: Option<&str>, archived: bool) -> Info {
    Info {
        id: id.into(),
        slug: "fixture".into(),
        project_id: "project".into(),
        directory: "/tmp/fixture".into(),
        parent_id: parent.map(str::to_owned),
        summary: None,
        share: None,
        title: "Fixture".into(),
        version: "test".into(),
        time: TimeInfo {
            created: 1,
            updated: 1,
            compacting: None,
            archived: archived.then_some(1),
        },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}

#[test]
fn archived_chat_delete_orders_descendants_before_root_and_preserves_unrelated() {
    let root = info("root", None, true);
    let sessions = vec![
        root.clone(),
        info("child", Some("root"), false),
        info("grandchild", Some("child"), false),
        info("unrelated", None, true),
    ];
    assert_eq!(delete_order(&root, &sessions).unwrap(), ["grandchild", "child", "root"]);
}

#[test]
fn archived_chat_delete_rejects_unarchived_root() {
    let root = info("root", None, false);
    assert!(delete_order(&root, std::slice::from_ref(&root)).is_err());
}

#[test]
fn archived_chat_delete_rejects_cross_project_descendant() {
    let root = info("root", None, true);
    let mut child = info("child", Some("root"), false);
    child.project_id = "other".into();
    assert!(delete_order(&root, &[root.clone(), child]).is_err());
}

#[test]
fn archived_chat_delete_rejects_cycle() {
    let root = info("root", Some("child"), true);
    let child = info("child", Some("root"), false);
    assert!(delete_order(&root, &[root.clone(), child]).is_err());
}

#[test]
fn archived_chat_delete_rejects_cross_directory_and_conflicting_ids() {
    let root = info("root", None, true);
    let mut child = info("child", Some("root"), false);
    child.directory = "/tmp/other".into();
    assert!(delete_order(&root, &[root.clone(), child]).is_err());
    let mut duplicate = root.clone();
    duplicate.project_id = "other".into();
    assert!(delete_order(&root, &[root.clone(), duplicate]).is_err());
}

#[test]
fn archived_chat_delete_rejects_hidden_root_and_deduplicates_identical_records() {
    let mut root = info("root", None, true);
    assert_eq!(delete_order(&root, &[root.clone(), root.clone()]).unwrap(), ["root"]);
    root.mark_agent_tool();
    assert!(delete_order(&root, std::slice::from_ref(&root)).is_err());
}
