use super::fixture::fixture;
#[test]
fn archived_chats_acceptance_fixture_preserves_large_list_and_duplicate_projects() {
    let rows = fixture();
    assert_eq!(rows.len(), 1178);
    assert_eq!(
        rows.iter().map(|row| &row.id).collect::<std::collections::HashSet<_>>().len(),
        1178
    );
    assert_eq!(
        rows.iter().map(|row| &row.project_id).collect::<std::collections::HashSet<_>>().len(),
        3
    );
    assert_eq!(rows[2].parent_id.as_deref(), Some("chat-0001"));
}
