//! Tenant-scoped business ledger. Private upload tickets never enter this representation.
use crate::global_database::GlobalDatabase;
use crate::global_database::config::DatabaseProvider;
use anyhow::{Context, Result, bail, ensure};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use vw_api_types::TenantScope;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoSourceIdentity {
    pub path: PathBuf,
    pub bytes: u64,
    pub modified_ns: u128,
    pub file_id: String,
    pub sha256: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudVideoScope {
    pub brand_id: Option<String>,
    pub store_id: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoPrepareState {
    New,
    CreatingProject,
    ProjectReady,
    CreatingUpload,
    UploadReady,
    Uploading,
    Finalizing,
    Probing,
    Ready,
    Rejected,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoRemoteJob {
    pub id: String,
    pub state: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoEditRunManifest {
    pub tenant: TenantScope,
    pub run_id: String,
    pub run_key: String,
    pub source: VideoSourceIdentity,
    pub template_sha256: String,
    pub model_sha256: String,
    pub requirements_sha256: String,
    pub cloud_scope: CloudVideoScope,
    pub project_id: Option<String>,
    pub upload_id: Option<String>,
    pub asset_id: Option<String>,
    pub jobs: Vec<VideoRemoteJob>,
    #[serde(default)]
    pub asset_jobs: Vec<super::assets::VideoAssetJob>,
    #[serde(default)]
    pub asset_peak: usize,
    #[serde(default)]
    pub asset_cancelled: bool,
    #[serde(default)]
    pub transcript: Option<super::transcript::VideoTranscriptState>,
    pub state: VideoPrepareState,
    pub revision: i64,
    pub lease_owner: String,
    pub lease_until_ms: i64,
}

const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS workflow_video_edit_runs (
 platform_id TEXT NOT NULL, company_id TEXT NOT NULL, run_key TEXT NOT NULL,
 payload TEXT NOT NULL, revision BIGINT NOT NULL, lease_owner TEXT NOT NULL,
 lease_until_ms BIGINT NOT NULL, PRIMARY KEY(platform_id, company_id, run_key));";
fn sqlite_sql(sql: &str) -> String {
    (1..=8).fold(sql.to_owned(), |sql, n| sql.replace(&format!("${n}"), &format!("?{n}")))
}
const CLAIM: &str =
    "UPDATE workflow_video_edit_runs SET lease_owner=$4, lease_until_ms=$5, revision=revision+1
 WHERE platform_id=$1 AND company_id=$2 AND run_key=$3 AND lease_until_ms<=$6
 RETURNING payload, revision";
const INSERT: &str = "INSERT INTO workflow_video_edit_runs
 (platform_id,company_id,run_key,payload,revision,lease_owner,lease_until_ms)
 VALUES($1,$2,$3,$4,0,'',0) ON CONFLICT(platform_id,company_id,run_key) DO NOTHING";
const SAVE: &str = "UPDATE workflow_video_edit_runs SET payload=$4,revision=revision+1,lease_until_ms=$5
 WHERE platform_id=$1 AND company_id=$2 AND run_key=$3 AND revision=$6 AND lease_owner=$7 AND lease_until_ms>$8";
const RELEASE: &str = "UPDATE workflow_video_edit_runs SET lease_owner='',lease_until_ms=0
 WHERE platform_id=$1 AND company_id=$2 AND run_key=$3 AND lease_owner=$4 AND revision=$5";
pub(super) const LEASE_MS: i64 = 15 * 60 * 1000;

/// Synchronous database operations; callers in an async runtime use an isolated thread.
#[derive(Clone)]
pub struct VideoEditManifestStore {
    database: GlobalDatabase,
}
impl VideoEditManifestStore {
    pub fn new(database: GlobalDatabase) -> Result<Self> {
        match database.provider() {
            DatabaseProvider::Sqlite => database.initialize_sqlite_schema(|c| {
                c.execute_batch(SCHEMA)?;
                // Never adopt an old or foreign table lacking the tenant key.
                let columns = c.prepare("PRAGMA table_info(workflow_video_edit_runs)")?
                    .query_map([], |r| Ok((r.get::<_, String>(1)?, r.get::<_, i64>(5)?)))?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                for (name, pk) in [("platform_id",1),("company_id",2),("run_key",3)] {
                    ensure!(columns.contains(&(name.into(),pk)), "unsafe video ledger schema");
                }
                Ok(())
            })?,
            DatabaseProvider::Postgres => database.initialize_postgres_schema(|c| {
                c.batch_execute(SCHEMA)?;
                let keys = c.query("SELECT a.attname FROM pg_index i JOIN pg_attribute a ON a.attrelid=i.indrelid AND a.attnum=ANY(i.indkey) WHERE i.indrelid='workflow_video_edit_runs'::regclass AND i.indisprimary", &[])?;
                let mut keys = keys.iter().map(|r| r.get::<_,String>(0)).collect::<Vec<_>>();
                keys.sort();
                ensure!(keys == ["company_id", "platform_id", "run_key"], "unsafe video ledger schema");
                Ok(())
            })?,
        }
        Ok(Self { database })
    }
    /// An expired lease may be taken, but pending side effects are NEVER reset to New.
    pub fn claim(
        &self,
        candidate: VideoEditRunManifest,
        owner: &str,
        now: i64,
    ) -> Result<VideoEditRunManifest> {
        ensure!(!owner.is_empty() && now >= 0, "invalid video lease");
        let until = now.checked_add(LEASE_MS).context("lease overflow")?;
        let p = candidate.tenant.platform_id().as_ref();
        let c = candidate.tenant.company_id().as_ref();
        let k = &candidate.run_key;
        let payload = serde_json::to_string(&candidate)?;
        let row: Option<(String, i64)> = match self.database.provider() {
            DatabaseProvider::Sqlite => self.database.with_sqlite_transaction(
                rusqlite::TransactionBehavior::Immediate,
                |tx| {
                    tx.execute(INSERT, params![p, c, k, payload])?;
                    Ok(tx
                        .query_row(&sqlite_sql(CLAIM), params![p, c, k, owner, until, now], |r| {
                            Ok((r.get(0)?, r.get(1)?))
                        })
                        .optional()?)
                },
            )?,
            DatabaseProvider::Postgres => {
                let mut connection = self.database.connect_postgres()?;
                let mut tx = connection.transaction()?;
                tx.execute(INSERT, &[&p, &c, &k, &payload])?;
                let row = tx
                    .query_opt(CLAIM, &[&p, &c, &k, &owner, &until, &now])?
                    .map(|r| (r.get(0), r.get(1)));
                tx.commit()?;
                row
            }
        };
        let Some((payload, revision)) = row else {
            bail!("video run busy: active lease");
        };
        let mut run: VideoEditRunManifest = serde_json::from_str(&payload)?;
        ensure!(
            run.tenant == candidate.tenant && run.run_key == *k,
            "video ledger identity mismatch"
        );
        ensure!(
            run.source == candidate.source
                && run.template_sha256 == candidate.template_sha256
                && run.model_sha256 == candidate.model_sha256
                && run.requirements_sha256 == candidate.requirements_sha256
                && run.cloud_scope == candidate.cloud_scope,
            "video ledger input mismatch"
        );
        run.revision = revision;
        run.lease_owner = owner.into();
        run.lease_until_ms = until;
        Ok(run)
    }
    pub fn save(&self, run: &mut VideoEditRunManifest, now: i64) -> Result<()> {
        let until = now.checked_add(LEASE_MS).context("lease overflow")?;
        let payload = serde_json::to_string(run)?;
        let p = run.tenant.platform_id().as_ref();
        let c = run.tenant.company_id().as_ref();
        let changed = match self.database.provider() {
            DatabaseProvider::Sqlite => self.database.connect_sqlite()?.execute(
                &sqlite_sql(SAVE),
                params![p, c, run.run_key, payload, until, run.revision, run.lease_owner, now],
            )? as u64,
            DatabaseProvider::Postgres => self.database.connect_postgres()?.execute(
                SAVE,
                &[&p, &c, &run.run_key, &payload, &until, &run.revision, &run.lease_owner, &now],
            )?,
        };
        ensure!(changed == 1, "video lease lost or revision changed");
        run.revision += 1;
        run.lease_until_ms = until;
        Ok(())
    }
    pub fn release(&self, run: &VideoEditRunManifest) -> Result<()> {
        let p = run.tenant.platform_id().as_ref();
        let c = run.tenant.company_id().as_ref();
        match self.database.provider() {
            DatabaseProvider::Sqlite => {
                self.database
                    .connect_sqlite()?
                    .execute(RELEASE, params![p, c, run.run_key, run.lease_owner, run.revision])?;
            }
            DatabaseProvider::Postgres => {
                self.database
                    .connect_postgres()?
                    .execute(RELEASE, &[&p, &c, &run.run_key, &run.lease_owner, &run.revision])?;
            }
        }
        Ok(())
    }
}
