use super::sessions::{OfficeCapability, OfficeSessions, OfficeSurface};
use std::time::{Duration, Instant};
use vw_api_types::computer_control::{
    ComputerControlAction, ComputerControlTarget, ControlDocumentId, ControlErrorCode,
    ControlSessionId, ExcelCommand, WordCommand,
};

fn owner() -> ControlSessionId {
    ControlSessionId("owner-a".into())
}

fn target(id: &str) -> ComputerControlTarget {
    ComputerControlTarget::ExcelWorkbook { document_id: ControlDocumentId(id.into()) }
}

fn read() -> ComputerControlAction {
    ComputerControlAction::Excel {
        command: ExcelCommand::ReadRange { sheet: "Sheet1".into(), range: "A1".into() },
    }
}

fn registry() -> OfficeSessions {
    OfficeSessions::new(Duration::from_secs(30)).unwrap()
}

fn register(
    sessions: &mut OfficeSessions,
    id: &str,
    now: Instant,
) -> super::sessions::OfficeHandle {
    sessions
        .register(
            owner(),
            OfficeSurface::Excel,
            ControlDocumentId(id.into()),
            "Untitled".into(),
            vec![OfficeCapability::ExcelReadRange],
            now,
        )
        .unwrap()
}

#[test]
fn cc_control_11_registration_discovers_identity_and_capabilities() {
    let now = Instant::now();
    let mut sessions = registry();
    let handle = register(&mut sessions, "book-a", now);
    let discovered = sessions.discover(&owner(), now);
    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].handle, handle);
    assert_eq!(discovered[0].title, "Untitled");
    assert_eq!(discovered[0].capabilities, vec![OfficeCapability::ExcelReadRange]);
    assert!(sessions.authorize(&owner(), &handle, &target("book-a"), &read(), now).is_ok());
}

#[test]
fn cc_control_11_heartbeat_extends_live_session_but_never_revives_expired_handle() {
    let now = Instant::now();
    let mut sessions = registry();
    let handle = register(&mut sessions, "book-a", now);
    assert!(sessions.heartbeat(&owner(), &handle, now + Duration::from_secs(29)).is_ok());
    assert!(
        sessions
            .authorize(&owner(), &handle, &target("book-a"), &read(), now + Duration::from_secs(31))
            .is_ok()
    );
    assert_eq!(
        sessions.heartbeat(&owner(), &handle, now + Duration::from_secs(59)),
        Err(ControlErrorCode::Disconnected)
    );
    assert!(sessions.discover(&owner(), now + Duration::from_secs(59)).is_empty());
}

#[test]
fn cc_control_11_same_title_documents_require_explicit_identity() {
    let now = Instant::now();
    let mut sessions = registry();
    let a = register(&mut sessions, "book-a", now);
    let b = register(&mut sessions, "book-b", now);
    assert_eq!(sessions.discover(&owner(), now).len(), 2);
    assert_eq!(
        sessions.authorize(&owner(), &a, &target("book-b"), &read(), now),
        Err(ControlErrorCode::TargetChanged)
    );
    assert!(sessions.authorize(&owner(), &b, &target("book-b"), &read(), now).is_ok());
}

#[test]
fn cc_control_11_reopen_rotates_generation_and_old_disconnect_cannot_close_new_session() {
    let now = Instant::now();
    let mut sessions = registry();
    let old = register(&mut sessions, "book-a", now);
    let new = register(&mut sessions, "book-a", now);
    assert_ne!(old.generation, new.generation);
    assert_eq!(
        sessions.authorize(&owner(), &old, &target("book-a"), &read(), now),
        Err(ControlErrorCode::Disconnected)
    );
    assert_eq!(sessions.disconnect(&owner(), &old), Err(ControlErrorCode::Disconnected));
    assert!(sessions.authorize(&owner(), &new, &target("book-a"), &read(), now).is_ok());
}

#[test]
fn cc_control_11_save_as_invalidates_old_document_and_close_rejects_commands() {
    let now = Instant::now();
    let mut sessions = registry();
    let old = register(&mut sessions, "book-a", now);
    let new = sessions
        .replace_document(
            &owner(),
            &old,
            ControlDocumentId("book-b".into()),
            "Untitled".into(),
            now,
        )
        .unwrap();
    assert_eq!(
        sessions.authorize(&owner(), &old, &target("book-a"), &read(), now),
        Err(ControlErrorCode::Disconnected)
    );
    assert!(sessions.authorize(&owner(), &new, &target("book-b"), &read(), now).is_ok());
    sessions.disconnect(&owner(), &new).unwrap();
    assert_eq!(
        sessions.authorize(&owner(), &new, &target("book-b"), &read(), now),
        Err(ControlErrorCode::Disconnected)
    );
}

#[test]
fn cc_control_11_owner_and_surface_cannot_be_substituted() {
    let now = Instant::now();
    let mut sessions = registry();
    let handle = register(&mut sessions, "shared", now);
    let other = ControlSessionId("owner-b".into());
    assert!(sessions.discover(&other, now).is_empty());
    assert_eq!(sessions.heartbeat(&other, &handle, now), Err(ControlErrorCode::PermissionDenied));
    assert_eq!(sessions.disconnect(&other, &handle), Err(ControlErrorCode::PermissionDenied));
    assert_eq!(
        sessions.authorize(&other, &handle, &target("shared"), &read(), now),
        Err(ControlErrorCode::PermissionDenied)
    );
    let word =
        ComputerControlTarget::WordDocument { document_id: ControlDocumentId("shared".into()) };
    let action = ComputerControlAction::Word { command: WordCommand::ReadDocument };
    assert_eq!(
        sessions.authorize(&owner(), &handle, &word, &action, now),
        Err(ControlErrorCode::TargetChanged)
    );
    sessions
        .register(
            other.clone(),
            OfficeSurface::Word,
            ControlDocumentId("shared".into()),
            "Untitled".into(),
            vec![OfficeCapability::WordReadDocument],
            now,
        )
        .unwrap();
    sessions.disconnect_owner(&owner());
    assert_eq!(sessions.discover(&other, now).len(), 1);
    assert!(sessions.discover(&owner(), now).is_empty());
}

#[test]
fn cc_control_11_only_exact_advertised_commands_are_allowed() {
    let now = Instant::now();
    let mut sessions = registry();
    let handle = register(&mut sessions, "book-a", now);
    let write = ComputerControlAction::Excel {
        command: ExcelCommand::WriteRange {
            sheet: "Sheet1".into(),
            range: "A1".into(),
            values: vec![],
        },
    };
    assert_eq!(
        sessions.authorize(&owner(), &handle, &target("book-a"), &write, now),
        Err(ControlErrorCode::Unsupported)
    );
    assert_eq!(
        sessions.authorize(&owner(), &handle, &target("book-a"), &ComputerControlAction::Stop, now),
        Err(ControlErrorCode::Unsupported)
    );
    assert!(
        sessions
            .register(
                owner(),
                OfficeSurface::Excel,
                ControlDocumentId("bad".into()),
                "Untitled".into(),
                vec![OfficeCapability::WordReadDocument],
                now
            )
            .is_err()
    );
}

#[test]
fn cc_control_11_invalid_registration_and_forged_handle_do_not_replace_live_document() {
    let now = Instant::now();
    let mut sessions = registry();
    assert!(OfficeSessions::new(Duration::ZERO).is_err());
    let handle = register(&mut sessions, "book-a", now);
    assert!(
        sessions
            .register(
                owner(),
                OfficeSurface::Excel,
                ControlDocumentId("book-a".into()),
                "x".repeat(513),
                vec![],
                now
            )
            .is_err()
    );
    assert!(
        sessions
            .register(
                owner(),
                OfficeSurface::Excel,
                ControlDocumentId(String::new()),
                "Untitled".into(),
                vec![],
                now
            )
            .is_err()
    );
    let mut forged = handle.clone();
    forged.document_id = ControlDocumentId("book-b".into());
    assert_eq!(sessions.heartbeat(&owner(), &forged, now), Err(ControlErrorCode::TargetChanged));
    assert_eq!(sessions.disconnect(&owner(), &forged), Err(ControlErrorCode::TargetChanged));
    assert!(sessions.authorize(&owner(), &handle, &target("book-a"), &read(), now).is_ok());
}

#[test]
fn cc_control_11_each_word_command_requires_its_own_advertisement() {
    use vw_api_types::computer_control::WordInsertPosition;
    let now = Instant::now();
    let mut sessions = registry();
    let cases = [
        (OfficeCapability::WordReadDocument, WordCommand::ReadDocument),
        (OfficeCapability::WordFindText, WordCommand::FindText { text: "needle".into() }),
        (
            OfficeCapability::WordReplaceText,
            WordCommand::ReplaceText { search: "a".into(), replacement: "b".into() },
        ),
        (
            OfficeCapability::WordInsertText,
            WordCommand::InsertText { position: WordInsertPosition::End, text: "text".into() },
        ),
    ];
    let target =
        ComputerControlTarget::WordDocument { document_id: ControlDocumentId("doc".into()) };
    for (capability, _) in &cases {
        let handle = sessions
            .register(
                owner(),
                OfficeSurface::Word,
                ControlDocumentId("doc".into()),
                "Untitled".into(),
                vec![*capability],
                now,
            )
            .unwrap();
        for (other, command) in &cases {
            let result = sessions.authorize(
                &owner(),
                &handle,
                &target,
                &ComputerControlAction::Word { command: command.clone() },
                now,
            );
            if capability == other {
                assert!(result.is_ok());
            } else {
                assert_eq!(result, Err(ControlErrorCode::Unsupported));
            }
        }
        sessions.disconnect(&owner(), &handle).unwrap();
        for (_, command) in &cases {
            assert_eq!(
                sessions.authorize(
                    &owner(),
                    &handle,
                    &target,
                    &ComputerControlAction::Word { command: command.clone() },
                    now
                ),
                Err(ControlErrorCode::Disconnected)
            );
        }
    }
}

#[test]
fn cc_control_11_expired_handle_rejects_both_excel_commands_and_invalid_save_as_closes_old_identity()
 {
    let now = Instant::now();
    let mut sessions = registry();
    let old = register(&mut sessions, "book-a", now);
    assert_eq!(
        sessions.replace_document(
            &owner(),
            &old,
            ControlDocumentId(String::new()),
            "Untitled".into(),
            now
        ),
        Err(ControlErrorCode::InvalidRequest)
    );
    assert_eq!(
        sessions.authorize(&owner(), &old, &target("book-a"), &read(), now),
        Err(ControlErrorCode::Disconnected)
    );
    let handle = sessions
        .register(
            owner(),
            OfficeSurface::Excel,
            ControlDocumentId("book-a".into()),
            "Untitled".into(),
            vec![OfficeCapability::ExcelReadRange, OfficeCapability::ExcelWriteRange],
            now,
        )
        .unwrap();
    let write = ComputerControlAction::Excel {
        command: ExcelCommand::WriteRange {
            sheet: "Sheet1".into(),
            range: "A1".into(),
            values: vec![],
        },
    };
    assert!(sessions.authorize(&owner(), &handle, &target("book-a"), &write, now).is_ok());
    for command in [read(), write] {
        assert_eq!(
            sessions.authorize(
                &owner(),
                &handle,
                &target("book-a"),
                &command,
                now + Duration::from_secs(30)
            ),
            Err(ControlErrorCode::Disconnected)
        );
    }
    assert_eq!(
        sessions.heartbeat(&owner(), &handle, now + Duration::from_secs(30)),
        Err(ControlErrorCode::Disconnected)
    );
}
