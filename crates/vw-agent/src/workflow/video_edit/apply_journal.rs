//! Durable tenant/project fence. An abandoned fence requires explicit reconciliation;
//! there is no expiry that could let a second writer race an unknown remote mutation.
use super::apply::{VideoApplyJournal, VideoApplyLedger};
use crate::global_database::{GlobalDatabase, config::DatabaseProvider, run_isolated};
use anyhow::{Result, ensure};
use rusqlite::{OptionalExtension, params};
use vw_api_types::TenantScope;

const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS workflow_video_project_writes (
 platform_id TEXT NOT NULL, company_id TEXT NOT NULL, project_id TEXT NOT NULL,
 payload TEXT NOT NULL, owner TEXT NOT NULL,
 PRIMARY KEY(platform_id,company_id,project_id));";
#[derive(Clone)]
pub struct VideoProjectJournal {
    database: GlobalDatabase,
    tenant: TenantScope,
    owner: String,
}
impl VideoProjectJournal {
    pub fn new(database: GlobalDatabase, tenant: TenantScope) -> Result<Self> {
        match database.provider() {
            DatabaseProvider::Sqlite => database.initialize_sqlite_schema(|c| {
                c.execute_batch(SCHEMA)?;
                let columns = c.prepare("PRAGMA table_info(workflow_video_project_writes)")?
                    .query_map([], |r| Ok((r.get::<_, String>(1)?, r.get::<_, i64>(5)?)))?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                for (name, pk) in [("platform_id",1),("company_id",2),("project_id",3)] {
                    ensure!(columns.contains(&(name.into(),pk)), "unsafe project writer schema");
                }
                Ok(())
            })?,
            DatabaseProvider::Postgres => database.initialize_postgres_schema(|c| {
                c.batch_execute(SCHEMA)?;
                let keys = c.query("SELECT a.attname FROM pg_index i JOIN pg_attribute a ON a.attrelid=i.indrelid AND a.attnum=ANY(i.indkey) WHERE i.indrelid='workflow_video_project_writes'::regclass AND i.indisprimary", &[])?;
                let mut keys = keys.iter().map(|r| r.get::<_,String>(0)).collect::<Vec<_>>();
                keys.sort();
                ensure!(keys == ["company_id", "platform_id", "project_id"], "unsafe project writer schema");
                Ok(())
            })?,
        }
        Ok(Self { database, tenant, owner: uuid::Uuid::new_v4().to_string() })
    }
    fn claim_sync(&self, project: &str) -> Result<VideoApplyLedger> {
        ensure!(!project.is_empty(), "missing project fence identity");
        let p = self.tenant.platform_id().as_ref();
        let c = self.tenant.company_id().as_ref();
        let initial = serde_json::to_string(&VideoApplyLedger::default())?;
        let payload: Option<String> = match self.database.provider() {
            DatabaseProvider::Sqlite => self.database.with_sqlite_transaction(rusqlite::TransactionBehavior::Immediate, |tx| {
                tx.execute("INSERT INTO workflow_video_project_writes VALUES(?1,?2,?3,?4,'') ON CONFLICT DO NOTHING", params![p,c,project,initial])?;
                Ok(tx.query_row("UPDATE workflow_video_project_writes SET owner=?4 WHERE platform_id=?1 AND company_id=?2 AND project_id=?3 AND owner='' RETURNING payload", params![p,c,project,self.owner], |r| r.get(0)).optional()?)
            })?,
            DatabaseProvider::Postgres => {
                let mut conn = self.database.connect_postgres()?;
                let mut tx = conn.transaction()?;
                tx.execute("INSERT INTO workflow_video_project_writes VALUES($1,$2,$3,$4,'') ON CONFLICT DO NOTHING", &[&p,&c,&project,&initial])?;
                let row = tx.query_opt("UPDATE workflow_video_project_writes SET owner=$4 WHERE platform_id=$1 AND company_id=$2 AND project_id=$3 AND owner='' RETURNING payload", &[&p,&c,&project,&self.owner])?.map(|r| r.get(0));
                tx.commit()?;
                row
            }
        };
        let payload = payload.ok_or_else(|| {
            anyhow::anyhow!("project writer busy or abandoned fence requires reconciliation")
        })?;
        Ok(serde_json::from_str(&payload)?)
    }
    fn update_sync(&self, project: &str, payload: Option<String>) -> Result<()> {
        let p = self.tenant.platform_id().as_ref();
        let c = self.tenant.company_id().as_ref();
        let changed = match (self.database.provider(), payload) {
            (DatabaseProvider::Sqlite, Some(payload)) => self.database.connect_sqlite()?.execute("UPDATE workflow_video_project_writes SET payload=?5 WHERE platform_id=?1 AND company_id=?2 AND project_id=?3 AND owner=?4", params![p,c,project,self.owner,payload])? as u64,
            (DatabaseProvider::Sqlite, None) => self.database.connect_sqlite()?.execute("UPDATE workflow_video_project_writes SET owner='' WHERE platform_id=?1 AND company_id=?2 AND project_id=?3 AND owner=?4", params![p,c,project,self.owner])? as u64,
            (DatabaseProvider::Postgres, Some(payload)) => self.database.connect_postgres()?.execute("UPDATE workflow_video_project_writes SET payload=$5 WHERE platform_id=$1 AND company_id=$2 AND project_id=$3 AND owner=$4", &[&p,&c,&project,&self.owner,&payload])?,
            (DatabaseProvider::Postgres, None) => self.database.connect_postgres()?.execute("UPDATE workflow_video_project_writes SET owner='' WHERE platform_id=$1 AND company_id=$2 AND project_id=$3 AND owner=$4", &[&p,&c,&project,&self.owner])?,
        };
        ensure!(changed == 1, "project writer fence lost");
        Ok(())
    }
}
#[async_trait::async_trait]
impl VideoApplyJournal for VideoProjectJournal {
    async fn claim(&self, project: &str) -> Result<VideoApplyLedger> {
        let this = self.clone();
        let project = project.to_owned();
        run_isolated(move || this.claim_sync(&project)).await?
    }
    async fn save(&self, project: &str, ledger: &VideoApplyLedger) -> Result<()> {
        let this = self.clone();
        let project = project.to_owned();
        let payload = serde_json::to_string(ledger)?;
        run_isolated(move || this.update_sync(&project, Some(payload))).await?
    }
    async fn release(&self, project: &str) -> Result<()> {
        let this = self.clone();
        let project = project.to_owned();
        run_isolated(move || this.update_sync(&project, None)).await?
    }
}
