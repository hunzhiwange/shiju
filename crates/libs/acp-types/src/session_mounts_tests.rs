use super::*;
use serde_json::json;

#[cfg(windows)]
const MOUNT_PATH: &str = r"C:\session-view\demo";
#[cfg(not(windows))]
const MOUNT_PATH: &str = "/tmp/demo";

#[test]
fn session_mounts_contract_rejects_unknown_version() {
    assert!(
        serde_json::from_value::<SessionMountSelection>(json!({"version":2,"skills":[]})).is_err()
    );
    assert!(
        serde_json::from_value::<SessionMountCapabilities>(
            json!({"version":2,"skills":true,"setMounts":false})
        )
        .is_err()
    );
}

#[test]
fn session_mounts_contract_rejects_invalid_mounts() {
    for mount in [
        json!({"name":"demo","path":"relative/demo"}),
        json!({"name":"demo","path":"/tmp/./demo"}),
        json!({"name":"demo","path":42}),
        json!({"name":"demo","path":"/tmp/../demo"}),
        json!({"name":"demo","path":"file:///tmp/demo"}),
        json!({"name":"../demo","path":MOUNT_PATH}),
        json!({"name":"","path":MOUNT_PATH}),
        json!({"name":"demo","path":"/tmp/demo\u{0}"}),
        json!({"name":"demo","path":MOUNT_PATH,"arguments":"secret"}),
    ] {
        assert!(
            serde_json::from_value::<SessionSkillMount>(mount.clone()).is_err(),
            "accepted {mount}"
        );
    }
}

#[test]
fn session_mounts_contract_rejects_duplicate_names() {
    let mount = json!({"name":"demo","path":MOUNT_PATH});
    assert!(
        serde_json::from_value::<SessionMountSelection>(
            json!({"version":1,"skills":[mount,mount]})
        )
        .is_err()
    );
}

#[test]
fn session_mounts_contract_requires_both_replacement_arrays() {
    for value in [
        json!({"sessionId":"s","expectedRevision":0,"skills":[]}),
        json!({"sessionId":"s","expectedRevision":0,"mcpServers":[]}),
        json!({"sessionId":"s","expectedRevision":0,"skills":null,"mcpServers":[]}),
        json!({"sessionId":"s","expectedRevision":0,"skills":[],"mcpServers":null}),
        json!({"sessionId":"s","expectedRevision":0,"skills":[],"mcpServers":[],"extra":true}),
    ] {
        assert!(serde_json::from_value::<SetSessionMountsRequest>(value).is_err());
    }
}

#[test]
fn session_mounts_contract_wire_round_trip() {
    let wire = json!({"sessionId":"s","expectedRevision":3,"skills":[{"name":"demo","path":MOUNT_PATH}],"mcpServers":[{"name":"fixture","command":"fixture-mcp","args":[],"env":[]}]});
    let parsed: SetSessionMountsRequest = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), wire);
    let reply = json!({"revision":4,"skills":["demo"],"mcpServers":["fixture"]});
    let parsed: SetSessionMountsResponse = serde_json::from_value(reply.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), reply);
}

#[test]
fn session_mounts_contract_missing_and_empty_selection_differ() {
    let saved: SessionSkillMount =
        serde_json::from_value(json!({"name":"中文_skill","path":MOUNT_PATH})).unwrap();
    let saved = vec![saved];
    assert!(SessionMountSelection::resolve(None, None).is_empty());
    assert_eq!(SessionMountSelection::resolve(None, Some(&saved)), saved);
    let meta =
        serde_json::from_value(json!({"shiju.skillMounts":{"version":1,"skills":[]}})).unwrap();
    let empty = SessionMountSelection::from_meta(Some(&meta)).unwrap().unwrap();
    assert!(SessionMountSelection::resolve(Some(&empty), Some(&saved)).is_empty());
    assert!(SessionMountSelection::from_meta(None).unwrap().is_none());
    let null = serde_json::from_value(json!({"shiju.skillMounts":null})).unwrap();
    assert!(SessionMountSelection::from_meta(Some(&null)).is_err());
}

#[test]
fn session_mounts_contract_guards_extension_by_advertised_capability() {
    let request: SetSessionMountsRequest = serde_json::from_value(
        json!({"sessionId":"s","expectedRevision":0,"skills":[],"mcpServers":[]}),
    )
    .unwrap();
    assert!(request.to_ext_request(None).is_err());
    assert!(request.to_ext_request(Some(&SessionMountCapabilities::default())).is_err());
    let meta = serde_json::from_value(
        json!({"shiju.mounts":{"version":1,"skills":true,"setMounts":true}}),
    )
    .unwrap();
    let capabilities = SessionMountCapabilities::from_meta(Some(&meta)).unwrap().unwrap();
    let ext = request.to_ext_request(Some(&capabilities)).unwrap();
    assert_eq!(&*ext.method, "shiju/session/set_mounts");
    let decoded: SetSessionMountsRequest = serde_json::from_str(ext.params.get()).unwrap();
    assert_eq!(decoded.expected_revision, SessionMountRevision(0));
    assert!(SessionMountCapabilities::from_meta(None).unwrap().is_none());
}

#[test]
fn session_mounts_contract_checks_revision_conflict_and_overflow() {
    assert_eq!(SessionMountRevision(3).next(SessionMountRevision(3)), Ok(SessionMountRevision(4)));
    assert_eq!(
        SessionMountRevision(3).next(SessionMountRevision(2)),
        Err(SessionMountError::RevisionConflict)
    );
    assert_eq!(
        SessionMountRevision(u64::MAX).next(SessionMountRevision(u64::MAX)),
        Err(SessionMountError::RevisionExhausted)
    );
    assert!(serde_json::from_value::<SessionMountRevision>(json!(-1)).is_err());
}

#[test]
fn session_mounts_contract_replacement_validates_duplicates_and_fields() {
    let mount = json!({"name":"demo","path":MOUNT_PATH});
    let server = json!({"name":"fixture","command":"fixture-mcp","args":[],"env":[]});
    for value in [
        json!({"sessionId":"s","expectedRevision":0,"skills":[mount,mount],"mcpServers":[]}),
        json!({"sessionId":"s","expectedRevision":0,"skills":[],"mcpServers":[server,server]}),
        json!({"sessionId":"s","expectedRevision":0,"skills":[],"mcpServers":[{"name":"bad"}]}),
    ] {
        assert!(serde_json::from_value::<SetSessionMountsRequest>(value).is_err());
    }
    for value in [
        json!({"version":1,"skills":[],"extra":true}),
        json!({"version":1}),
        json!({"version":1,"skills":null}),
    ] {
        assert!(serde_json::from_value::<SessionMountSelection>(value).is_err());
    }
    assert!(
        serde_json::from_value::<SessionMountCapabilities>(
            json!({"version":1,"skills":true,"setMounts":false,"extra":true})
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<SetSessionMountsResponse>(
            json!({"revision":1,"skills":["demo","demo"],"mcpServers":[]})
        )
        .is_err()
    );
}
