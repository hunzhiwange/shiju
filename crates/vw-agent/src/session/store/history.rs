//! Ordinary UI writes may extend a transcript, but cannot replace its identity or
//! discard rows. Explicit, version-checked delta truncation remains available.
use anyhow::Result;
use postgres::GenericClient;
use rusqlite::{Connection, OptionalExtension, params};
use vw_api_types::id::ProjectScope;

use super::{ChatMessage, ChatRole, ChatSession, SessionMessageDelta, SessionMessageDeltaError};

fn validate_message(
    seq_no: i64,
    stored: (&str, Option<&str>, &str),
    incoming: Option<(&ChatMessage, Option<&str>)>,
) -> Result<()> {
    let (role, message_id, content) = stored;
    let valid = incoming.is_some_and(|(message, incoming_id)| {
        super::role_to_db(message.role) == role
            && message_id.is_none_or(|id| Some(id) == incoming_id)
            // User/system/tool rows identify the prior context, even for legacy
            // transcripts without IDs. Assistant text can be finalized by runtime.
            && (message.role == ChatRole::Assistant || message.content == content)
    });
    if !valid {
        return Err(SessionMessageDeltaError::Conflict(format!(
            "session history conflict at seq_no {seq_no}: reload history before saving"
        ))
        .into());
    }
    Ok(())
}

pub(super) fn validate_sqlite_snapshot(
    connection: &Connection,
    scope: &ProjectScope,
    session: &ChatSession,
) -> Result<()> {
    let mut statement = connection.prepare(
        "SELECT seq_no, role, message_id, content FROM session_messages
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND session_id = ?4",
    )?;
    let mut rows = statement.query(super::scoped_id_params(scope, &session.id))?;
    while let Some(row) = rows.next()? {
        let seq_no: i64 = row.get(0)?;
        let index = super::from_i64_usize(seq_no, "message seq_no")?;
        let role: String = row.get(1)?;
        let id: Option<String> = row.get(2)?;
        let content: String = row.get(3)?;
        validate_message(
            seq_no,
            (&role, id.as_deref(), &content),
            session.messages.get(index).map(|message| {
                (message, session.message_ids.get(index).and_then(Option::as_deref))
            }),
        )?;
    }
    Ok(())
}

pub(super) fn validate_postgres_snapshot(
    client: &mut impl GenericClient,
    scope: &ProjectScope,
    session: &ChatSession,
) -> Result<()> {
    // Delta writers lock this header too. Hold it before checking the rows so
    // an append cannot commit between validation and snapshot replacement.
    client.query_opt(
        "SELECT 1 FROM session_index
         WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND session_id = $4
         FOR UPDATE",
        &[
            &scope.platform_id().as_ref(),
            &scope.company_id().as_ref(),
            &scope.project_id().as_ref(),
            &session.id,
        ],
    )?;
    for row in client.query(
        "SELECT seq_no, role, message_id, content FROM session_messages
         WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND session_id = $4",
        &[
            &scope.platform_id().as_ref(),
            &scope.company_id().as_ref(),
            &scope.project_id().as_ref(),
            &session.id,
        ],
    )? {
        let seq_no: i64 = row.try_get(0)?;
        let index = super::from_i64_usize(seq_no, "message seq_no")?;
        validate_message(
            seq_no,
            (row.try_get(1)?, row.try_get(2)?, row.try_get(3)?),
            session.messages.get(index).map(|message| {
                (message, session.message_ids.get(index).and_then(Option::as_deref))
            }),
        )?;
    }
    Ok(())
}

pub(super) fn validate_sqlite_delta(
    connection: &Connection,
    scope: &ProjectScope,
    delta: &SessionMessageDelta,
) -> Result<()> {
    let mut statement = connection.prepare(
        "SELECT role, message_id, content FROM session_messages
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
           AND session_id = ?4 AND seq_no = ?5",
    )?;
    for record in &delta.upserts {
        // The caller explicitly authorized replacing the truncated tail.
        if delta.truncate_after.is_some_and(|boundary| record.seq_no > boundary) {
            continue;
        }
        let seq_no = super::to_i64(record.seq_no, "message seq_no")?;
        let stored = statement
            .query_row(
                params![
                    scope.platform_id().as_ref(),
                    scope.company_id().as_ref(),
                    scope.project_id().as_ref(),
                    delta.session_id,
                    seq_no
                ],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?;
        if let Some((role, id, content)) = stored {
            validate_message(
                seq_no,
                (&role, id.as_deref(), &content),
                Some((&record.message, record.message_id.as_deref())),
            )?;
        }
    }
    Ok(())
}

pub(super) fn validate_postgres_delta(
    client: &mut impl GenericClient,
    scope: &ProjectScope,
    delta: &SessionMessageDelta,
) -> Result<()> {
    for record in &delta.upserts {
        // The caller explicitly authorized replacing the truncated tail.
        if delta.truncate_after.is_some_and(|boundary| record.seq_no > boundary) {
            continue;
        }
        let seq_no = super::to_i64(record.seq_no, "message seq_no")?;
        if let Some(row) = client.query_opt(
            "SELECT role, message_id, content FROM session_messages
             WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
               AND session_id = $4 AND seq_no = $5",
            &[
                &scope.platform_id().as_ref(),
                &scope.company_id().as_ref(),
                &scope.project_id().as_ref(),
                &delta.session_id,
                &seq_no,
            ],
        )? {
            validate_message(
                seq_no,
                (row.try_get(0)?, row.try_get(1)?, row.try_get(2)?),
                Some((&record.message, record.message_id.as_deref())),
            )?;
        }
    }
    Ok(())
}
