use super::*;
use crate::model_relay::RelayIdentity;

#[test]
fn missing_anchor_recovers_only_the_latest_matching_managed_generation() {
    let launch = RelayIdentity {
        tenant: vw_api_types::id::TenantScope::default(),
        project: "project".into(),
        session: ":claude".into(),
        provider: "deepseek".into(),
        model: "deepseek-flash".into(),
        generation: 0,
    };
    let entry = |generation, directory: &str, provider: &str, closed| {
        let scoped = format!("local-session:acp-source-{generation}:tenant-and-directory");
        let mut identity = launch.clone();
        identity.session = format!("{scoped}:claude");
        identity.provider = provider.into();
        vw_acp::SessionIndexEntry {
            file: format!("record-{generation}.json"),
            vwacp_record_id: format!("record-{generation}"),
            acp_session_id: format!("acp-{generation}"),
            agent_command: launch_reference(&identity).unwrap(),
            cwd: directory.into(),
            name: Some(format!("vw-session:{scoped}")),
            closed,
            last_used_at: String::new(),
        }
    };
    let mut entries = vec![
        entry(3, "/workspace", "other-provider", false),
        entry(2, "/other", "deepseek", false),
        entry(1, "/workspace", "deepseek", true),
    ];
    let recover = |entries: &[vw_acp::SessionIndexEntry]| {
        recover_from_index(
            entries,
            "local-session",
            "tenant-and-directory",
            "/workspace",
            3,
            &Launch::Managed(launch.clone()),
        )
        .map(|result| result.map(|(generation, _)| generation))
    };
    assert!(recover(&entries[..]).is_err());
    entries.push(entry(0, "/workspace", "deepseek", false));
    assert_eq!(recover(&entries[..]).unwrap(), Some(0));
    entries.push(entry(3, "/workspace", "deepseek", false));
    assert_eq!(recover(&entries[..]).unwrap(), Some(3));
}

#[test]
fn local_recovery_rejects_unverified_or_ambiguous_history() {
    let config = AcpAgentConfig { command: "local-agent".into(), ..Default::default() };
    let launch = Launch::Local { agent: "local-agent", config: &config };
    let record = SessionIndexEntry {
        file: "original.json".into(),
        vwacp_record_id: "original".into(),
        acp_session_id: "native".into(),
        agent_command: "local-agent".into(),
        cwd: "/workspace".into(),
        name: Some("vw-session:chat:acp-source-0:scope".into()),
        closed: false,
        last_used_at: String::new(),
    };
    let find = |entries: &[SessionIndexEntry]| {
        recover_from_index(entries, "chat", "scope", "/workspace", 0, &launch)
            .map(|found| found.map(|(_, entry)| entry.vwacp_record_id.clone()))
    };
    assert_eq!(find(std::slice::from_ref(&record)).unwrap().as_deref(), Some("original"));
    let mut foreign = record.clone();
    foreign.name = Some("vw-session:chat:acp-source-0:other-scope".into());
    assert_eq!(find(&[foreign]).unwrap(), None);
    let mut invalid = record.clone();
    invalid.acp_session_id = " ".into();
    assert!(find(&[invalid]).is_err());
    let mut duplicate = record.clone();
    duplicate.vwacp_record_id = "duplicate".into();
    assert!(find(&[record, duplicate]).is_err());
}
