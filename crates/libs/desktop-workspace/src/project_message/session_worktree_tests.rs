#[test]
fn picker_loaded_reorders_last_directory_after_primary_workspace() {
    let worktree = "/tmp/project-wt".to_string();

    let mut options = vec![
        ("/tmp/other".to_string(), "其它".to_string()),
        (worktree.clone(), "最近".to_string()),
        ("/tmp/project".to_string(), "主工作区".to_string()),
    ];
    super::order_options(&mut options, Some(&worktree));

    assert_eq!(
        options,
        vec![
            (worktree, "最近".to_string()),
            ("/tmp/other".to_string(), "其它".to_string()),
            ("/tmp/project".to_string(), "主工作区".to_string()),
        ]
    );
}

#[test]
fn picker_loaded_reorders_last_directory_after_first_primary_workspace() {
    let worktree = "/tmp/project-wt".to_string();

    let mut options = vec![
        ("/tmp/project".to_string(), "主工作区".to_string()),
        ("/tmp/other".to_string(), "其它".to_string()),
        (worktree.clone(), "最近".to_string()),
    ];
    super::order_options(&mut options, Some(&worktree));

    assert_eq!(options[0].1, "主工作区");
    assert_eq!(options[1].0, worktree);
}
