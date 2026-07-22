use super::WORKER_COLUMNS;

#[test]
fn worker_columns_match_map_worker_row_positional_indexes() {
    let columns = WORKER_COLUMNS.split(", ").collect::<Vec<_>>();

    assert_eq!(
        columns,
        vec![
            "id",
            "runtime",
            "deployment",
            "model",
            "model_provider",
            "status",
            "desired_state",
            "team_id",
            "worker_role",
            "matrix_user_id",
            "room_id",
            "container_state",
            "idle_since_ms",
            "last_seen_at_ms",
            "last_heartbeat_at",
            "capabilities_json",
            "updated_at_ms",
        ]
    );
}
