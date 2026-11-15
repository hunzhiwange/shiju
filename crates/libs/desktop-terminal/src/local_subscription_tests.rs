#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn terminal_subscription_project_switch_distinguishes_reused_tab_ids() {
    use iced::advanced::subscription::{Hasher, into_recipes};
    use std::hash::Hasher as _;
    let mut first = crate::TerminalState::default();
    let mut second = crate::TerminalState::default();
    first.tabs.clear();
    second.tabs.clear();
    let first_id = first
        .add_terminal_with_command("first".into(), Some(std::env::temp_dir()), "exec cat".into())
        .unwrap();
    let second_id = second
        .add_terminal_with_command("second".into(), Some(std::env::temp_dir()), "exec cat".into())
        .unwrap();
    assert_eq!(first_id, second_id);
    let hash = |term: &iced_term::Terminal| {
        let mut recipes = into_recipes(super::subscription(term));
        assert_eq!(recipes.len(), 1);
        let mut hasher = Hasher::default();
        recipes.pop().unwrap().hash(&mut hasher);
        hasher.finish()
    };
    assert_eq!(hash(&first.tabs[0].term), hash(&first.tabs[0].term));
    assert_ne!(
        hash(&first.tabs[0].term),
        hash(&second.tabs[0].term),
        "project switch must replace the PTY receiver instead of retaining the old recipe"
    );
}
