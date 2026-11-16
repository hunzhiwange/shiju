//! Controlled remote receipts with a caller-owned context port.
use crate::{manifest::*, prepare::*, cloud_failure};
use anyhow::{Result, bail};
use std::sync::{Arc, Mutex};
#[derive(Default)]
pub struct Remote {
    pub projects: usize,
    pub sessions: usize,
    pub puts: usize,
    pub finalizes: usize,
    pub project: Option<String>,
    pub fail: String,
    pub nonvideo: bool,
    pub incomplete: bool,
    pub replace: Option<std::path::PathBuf>,
}
#[derive(Clone)]
pub struct Cloud<C> {
    pub ctx: C,
    pub scopes: Vec<CloudVideoScope>,
    pub remote: Arc<Mutex<Remote>>,
}
#[async_trait::async_trait]
impl<C: crate::context::VideoContext + Clone + 'static> VideoPrepareCloud for Cloud<C> {
    fn context(&self) -> crate::context::VideoContextHandle {
        std::sync::Arc::new(self.ctx.clone())
    }
    async fn authorized_scopes(&self) -> Result<Vec<CloudVideoScope>> {
        Ok(self.scopes.clone())
    }
    async fn create_project(&self, _run: &str, _scope: &CloudVideoScope) -> Result<String> {
        let mut r = self.remote.lock().unwrap();
        if r.fail == "not-sent" || r.fail == "rejected" {
            let mut error = cloud_failure::CloudOperationFailure::not_sent(
                cloud_failure::CloudFailurePhase::Preflight,
            );
            if r.fail == "rejected" {
                error.outcome = cloud_failure::CloudFailureOutcome::Rejected;
            }
            r.fail.clear();
            return Err(error.into());
        }
        r.projects += 1;
        if r.fail == "unknown-no-project" {
            r.fail.clear();
            return Err(cloud_failure::CloudOperationFailure::unknown(
                cloud_failure::CloudFailurePhase::Response,
            )
            .into());
        }
        let id = format!("project-{}", r.projects);
        r.project = Some(id.clone());
        if r.fail == "project" {
            r.fail.clear();
            bail!("connection interrupted after create");
        }
        Ok(id)
    }
    async fn find_project(&self, _run: &str, _scope: &CloudVideoScope) -> Result<Option<String>> {
        Ok(self.remote.lock().unwrap().project.clone())
    }
    async fn create_upload(
        &self,
        _project: &str,
        _source: &VideoSourceIdentity,
    ) -> Result<VideoUploadTicket> {
        let mut r = self.remote.lock().unwrap();
        if r.fail == "upload-not-sent" || r.fail == "upload-rejected" {
            let mut failure = cloud_failure::CloudOperationFailure::not_sent(
                cloud_failure::CloudFailurePhase::Preflight,
            );
            if r.fail == "upload-rejected" {
                failure.outcome = cloud_failure::CloudFailureOutcome::Rejected;
            }
            r.fail.clear();
            return Err(failure.into());
        }
        r.sessions += 1;
        if let Some(path) = r.replace.take() {
            std::fs::write(path, b"replacement source").unwrap();
        }
        if r.fail == "session" {
            r.fail.clear();
            bail!("session response lost");
        }
        Ok(VideoUploadTicket {
            upload_id: format!("upload-{}", r.sessions),
            url: "https://fixture.oss.aliyuncs.com/object?Signature=private".into(),
            headers: Default::default(),
        })
    }
    async fn upload(
        &self,
        _ticket: &VideoUploadTicket,
        _source: &VideoSourceIdentity,
    ) -> Result<()> {
        let mut r = self.remote.lock().unwrap();
        r.puts += 1;
        if r.fail == "put-incomplete" {
            r.fail.clear();
            r.incomplete = true;
            bail!("PUT interrupted before completion");
        }
        if r.fail == "put" {
            r.fail.clear();
            bail!("PUT response lost");
        }
        Ok(())
    }
    async fn finalize(
        &self,
        _project: &str,
        _upload: &str,
        job: Option<&str>,
    ) -> Result<VideoImportResult> {
        let mut r = self.remote.lock().unwrap();
        r.finalizes += 1;
        if r.incomplete {
            bail!("remote upload incomplete; reconciliation required");
        }
        if r.fail == "finalize" {
            r.fail.clear();
            bail!("finalize response lost");
        }
        if r.fail == "pending" {
            r.fail.clear();
            return Ok(VideoImportResult::Pending("job-original".into()));
        }
        if let Some(job) = job {
            assert_eq!(job, "job-original");
        }
        Ok(VideoImportResult::Ready("asset-original".into()))
    }
    async fn probe(&self, _project: &str, _asset: &str) -> Result<VideoMediaInfo> {
        Ok(VideoMediaInfo {
            has_video: !self.remote.lock().unwrap().nonvideo,
            width: 1920,
            height: 1080,
            duration_sec: 45.0,
            has_audio: true,
        })
    }
}
