//! Add user ownership to existing session tables. The column default assigns legacy rows to user 0.

use std::collections::HashSet;

use anyhow::{Context, Result};
use rusqlite::{Connection, Transaction, TransactionBehavior};

use super::{FIRST_RESPONSE_SCHEMA, SESSION_SCHEMA};
use crate::dynamic_tool_store;

// Parent tables must be copied first and dropped last.
const TABLES: &[&str] = &[
    "session_index",
    "session_agent_sessions",
    "session_first_response",
    "session_messages",
    "session_calls",
    "session_steps",
    "session_todos",
    "session_step_llm_raw",
    "session_llm_raw_turns",
    "session_llm_raw_rounds",
    "session_dynamic_tools",
];

pub(super) fn initialize_sqlite(connection: &Connection) -> Result<()> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    let mut legacy = Vec::new();
    for &table in TABLES {
        let columns = sqlite_columns(&transaction, table)?;
        if !columns.is_empty() && !columns.contains("user_id") {
            transaction.execute_batch(&format!(
                "ALTER TABLE {table} RENAME TO {table}_legacy_user_scope"
            ))?;
            legacy.push((table, columns));
        }
    }
    let create = || {
        transaction.execute_batch(SESSION_SCHEMA)?;
        transaction.execute_batch(FIRST_RESPONSE_SCHEMA)?;
        transaction.execute_batch(dynamic_tool_store::SCHEMA)
    };
    create().context("initialize SQLite session tables")?;
    if !legacy.is_empty() {
        for (table, columns) in &legacy {
            let target_columns = sqlite_columns(&transaction, table)?;
            transaction.execute_batch(&copy_legacy_rows(table, columns, target_columns))?;
        }
        for (table, _) in legacy.iter().rev() {
            transaction.execute_batch(&format!("DROP TABLE {table}_legacy_user_scope"))?;
        }
        // Index names on renamed legacy tables prevented their creation above.
        create().context("initialize SQLite user-scoped session indexes")?;
    }
    transaction.commit().context("commit SQLite session schema")
}

pub(super) fn initialize_postgres(transaction: &mut ::postgres::Transaction<'_>) -> Result<()> {
    let mut legacy = Vec::new();
    for &table in TABLES {
        let columns = postgres_columns(transaction, table)?;
        if !columns.is_empty() && !columns.contains("user_id") {
            transaction.batch_execute(&format!(
                "ALTER TABLE {table} RENAME TO {table}_legacy_user_scope"
            ))?;
            legacy.push((table, columns));
        }
    }
    create_postgres_schema(transaction)?;
    if !legacy.is_empty() {
        for (table, columns) in &legacy {
            let target_columns = postgres_columns(transaction, table)?;
            transaction.batch_execute(&copy_legacy_rows(table, columns, target_columns))?;
        }
        for (table, _) in legacy.iter().rev() {
            transaction.batch_execute(&format!("DROP TABLE {table}_legacy_user_scope"))?;
        }
        create_postgres_schema(transaction)?;
    }
    Ok(())
}

fn create_postgres_schema(transaction: &mut ::postgres::Transaction<'_>) -> Result<()> {
    transaction.batch_execute(super::postgres::SESSION_SCHEMA)?;
    transaction.batch_execute(FIRST_RESPONSE_SCHEMA)?;
    transaction.batch_execute(dynamic_tool_store::SCHEMA)?;
    Ok(())
}

fn postgres_columns(
    transaction: &mut ::postgres::Transaction<'_>,
    table: &str,
) -> Result<HashSet<String>> {
    let rows = transaction.query(
        "SELECT column_name FROM information_schema.columns
         WHERE table_schema = current_schema() AND table_name = $1",
        &[&table],
    )?;
    Ok(rows.into_iter().map(|row| row.get(0)).collect())
}

fn sqlite_columns(connection: &Connection, table: &str) -> Result<HashSet<String>> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn copy_legacy_rows(
    table: &str,
    legacy_columns: &HashSet<String>,
    target_columns: HashSet<String>,
) -> String {
    let mut shared = target_columns.intersection(legacy_columns).collect::<Vec<_>>();
    shared.sort_unstable();
    let fields = shared.iter().map(|name| name.as_str()).collect::<Vec<_>>();
    let values = fields
        .iter()
        .map(|&name| {
            if table == "session_steps" && name == "turn_id" {
                "COALESCE(turn_id, '')"
            } else {
                name
            }
        })
        .collect::<Vec<_>>();
    format!(
        "INSERT INTO {table} ({}) SELECT {} FROM {table}_legacy_user_scope",
        fields.join(", "),
        values.join(", "),
    )
}
