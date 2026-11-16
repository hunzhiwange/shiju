use crate::acp_update::sort_acp_agent_names;
#[test]
fn sort_acp_agent_names_keeps_codex_first_and_sorts_remaining_names() {
    let names = vec!["zeta".to_string(), "codex".to_string(), "alpha".to_string()];

    assert_eq!(sort_acp_agent_names(names), ["codex", "alpha", "zeta"]);
}
