use super::PostgresMemory;
use super::tls::NoCertVerifier;
use anyhow::{Context, Result};
use postgres::{Client, NoTls};
use std::time::Duration;

/// PostgreSQL 连接超时时间的上限（秒）。
const POSTGRES_CONNECT_TIMEOUT_CAP_SECS: u64 = 300;

impl PostgresMemory {
    /// 在独立线程中初始化客户端并确保 schema 存在。
    pub(super) fn initialize_client(
        db_url: String,
        connect_timeout_secs: Option<u64>,
        tls_mode: bool,
        schema_ident: String,
        qualified_table: String,
    ) -> Result<Client> {
        let init_handle = std::thread::Builder::new()
            .name("postgres-memory-init".to_string())
            .spawn(move || -> Result<Client> {
                let mut config: postgres::Config =
                    db_url.parse().context("invalid PostgreSQL connection URL")?;

                if let Some(timeout_secs) = connect_timeout_secs {
                    config.connect_timeout(Self::bounded_connect_timeout(timeout_secs));
                }

                let mut client = if tls_mode {
                    let tls_config = rustls::ClientConfig::builder()
                        .with_root_certificates(rustls::RootCertStore::empty())
                        .with_no_client_auth();

                    let tls_config = {
                        let mut cfg = tls_config;
                        cfg.dangerous()
                            .set_certificate_verifier(std::sync::Arc::new(NoCertVerifier));
                        cfg
                    };

                    let tls = tokio_postgres_rustls::MakeRustlsConnect::new(tls_config);
                    config
                        .connect(tls)
                        .context("failed to connect to PostgreSQL memory backend (TLS)")?
                } else {
                    config
                        .connect(NoTls)
                        .context("failed to connect to PostgreSQL memory backend")?
                };

                Self::init_schema(&mut client, &schema_ident, &qualified_table)?;
                Ok(client)
            })
            .context("failed to spawn PostgreSQL initializer thread")?;

        init_handle.join().map_err(|_| anyhow::anyhow!("PostgreSQL initializer thread panicked"))?
    }

    /// 初始化 schema、表与索引。
    pub(super) fn init_schema(
        client: &mut Client,
        schema_ident: &str,
        qualified_table: &str,
    ) -> Result<()> {
        client.batch_execute(&Self::schema_sql(schema_ident, qualified_table))?;

        Ok(())
    }

    fn bounded_connect_timeout(timeout_secs: u64) -> Duration {
        Duration::from_secs(timeout_secs.min(POSTGRES_CONNECT_TIMEOUT_CAP_SECS))
    }

    fn schema_sql(schema_ident: &str, qualified_table: &str) -> String {
        format!(
            "
            CREATE SCHEMA IF NOT EXISTS {schema_ident};

            CREATE TABLE IF NOT EXISTS {qualified_table} (
                id TEXT PRIMARY KEY,
                key TEXT UNIQUE NOT NULL,
                content TEXT NOT NULL,
                category TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                session_id TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_memories_category ON {qualified_table}(category);
            CREATE INDEX IF NOT EXISTS idx_memories_session_id ON {qualified_table}(session_id);
            CREATE INDEX IF NOT EXISTS idx_memories_updated_at ON {qualified_table}(updated_at DESC);
            "
        )
    }
}

#[cfg(test)]
#[path = "backend_tests.rs"]
mod backend_tests;
