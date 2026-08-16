//! Gateway 内存中的待提交 LLM 原始工件。
//!
//! 工件在处理器完成一个 LLM 步骤后先登记于此；UI 快照保存成功后才按精确版本确认。

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use anyhow::{Result, bail};
use vw_api_types::id::TenantScope;

use crate::session::llm_raw::LlmRawArtifact;
use crate::session::store::LlmRawStepKey;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RawCommitKey {
    platform_id: String,
    company_id: String,
    project_id: String,
    session_id: String,
    turn_id: String,
}

#[derive(Debug, Clone)]
struct PendingArtifact {
    version: u64,
    artifact: LlmRawArtifact,
}

#[derive(Debug, Clone)]
struct ClaimedArtifact {
    key: RawCommitKey,
    step_index: u32,
    version: u64,
}

/// 一次 UI 保存所声明的、尚未确认的原始工件快照。
#[derive(Debug, Clone)]
pub(super) struct RawCommitClaims {
    raw_artifacts: Vec<LlmRawArtifact>,
    expected_steps: Vec<LlmRawStepKey>,
    claimed: Vec<ClaimedArtifact>,
}

impl RawCommitClaims {
    pub(super) fn is_empty(&self) -> bool {
        self.claimed.is_empty()
    }

    pub(super) fn raw_artifacts(&self) -> &[LlmRawArtifact] {
        &self.raw_artifacts
    }

    pub(super) fn expected_steps(&self) -> &[LlmRawStepKey] {
        &self.expected_steps
    }
}

#[derive(Debug, Default)]
pub(super) struct PendingRawCommitRegistry {
    next_version: u64,
    entries: HashMap<RawCommitKey, HashMap<u32, PendingArtifact>>,
}

impl PendingRawCommitRegistry {
    pub(super) fn register(
        &mut self,
        tenant: &TenantScope,
        project_id: Option<&str>,
        artifact: LlmRawArtifact,
    ) -> Result<()> {
        let project_id = required_project_id(project_id)?;
        if artifact.context.session_id.trim().is_empty() {
            bail!("raw LLM artifact requires a session id");
        }
        if artifact.context.turn_id.trim().is_empty() {
            bail!("raw LLM artifact requires a turn id");
        }
        for round in &artifact.rounds {
            if round.turn_id != artifact.context.turn_id {
                bail!("raw LLM artifact round turn id does not match its context");
            }
        }

        let key = RawCommitKey {
            platform_id: tenant.platform_id().as_ref().to_string(),
            company_id: tenant.company_id().as_ref().to_string(),
            project_id: project_id.to_string(),
            session_id: artifact.context.session_id.clone(),
            turn_id: artifact.context.turn_id.clone(),
        };
        for round in &artifact.rounds {
            self.next_version = self.next_version.saturating_add(1).max(1);
            self.entries.entry(key.clone()).or_default().insert(
                round.step_index,
                PendingArtifact { version: self.next_version, artifact: artifact.clone() },
            );
        }
        Ok(())
    }

    pub(super) fn claim(
        &self,
        tenant: &TenantScope,
        project_id: Option<&str>,
        session_id: &str,
        finished_steps: &[(String, u32)],
    ) -> Result<RawCommitClaims> {
        let project_id = required_project_id(project_id)?;
        let mut raw_artifacts = Vec::new();
        let mut expected_steps = Vec::new();
        let mut claimed = Vec::new();
        for (turn_id, step_index) in finished_steps {
            let key = RawCommitKey {
                platform_id: tenant.platform_id().as_ref().to_string(),
                company_id: tenant.company_id().as_ref().to_string(),
                project_id: project_id.to_string(),
                session_id: session_id.to_string(),
                turn_id: turn_id.clone(),
            };
            let Some(entry) = self.entries.get(&key).and_then(|rounds| rounds.get(step_index))
            else {
                continue;
            };
            raw_artifacts.push(entry.artifact.clone());
            expected_steps.push(LlmRawStepKey::new(turn_id, *step_index)?);
            claimed.push(ClaimedArtifact { key, step_index: *step_index, version: entry.version });
        }
        Ok(RawCommitClaims { raw_artifacts, expected_steps, claimed })
    }

    pub(super) fn acknowledge(&mut self, claims: &RawCommitClaims) {
        for claimed in &claims.claimed {
            let remove_turn = if let Some(rounds) = self.entries.get_mut(&claimed.key) {
                let is_claimed_version = rounds
                    .get(&claimed.step_index)
                    .is_some_and(|entry| entry.version == claimed.version);
                if is_claimed_version {
                    rounds.remove(&claimed.step_index);
                }
                rounds.is_empty()
            } else {
                false
            };
            if remove_turn {
                self.entries.remove(&claimed.key);
            }
        }
    }
}

fn required_project_id(project_id: Option<&str>) -> Result<&str> {
    project_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("raw LLM artifact requires a project scope"))
}

static PENDING_RAW_COMMITS: LazyLock<Mutex<PendingRawCommitRegistry>> =
    LazyLock::new(|| Mutex::new(PendingRawCommitRegistry::default()));

pub(super) fn register(
    tenant: &TenantScope,
    project_id: Option<&str>,
    artifact: LlmRawArtifact,
) -> Result<()> {
    PENDING_RAW_COMMITS
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .register(tenant, project_id, artifact)
}

pub(super) fn claim(
    tenant: &TenantScope,
    project_id: Option<&str>,
    session_id: &str,
    finished_steps: &[(String, u32)],
) -> Result<RawCommitClaims> {
    PENDING_RAW_COMMITS.lock().unwrap_or_else(|error| error.into_inner()).claim(
        tenant,
        project_id,
        session_id,
        finished_steps,
    )
}

pub(super) fn acknowledge(claims: &RawCommitClaims) {
    PENDING_RAW_COMMITS.lock().unwrap_or_else(|error| error.into_inner()).acknowledge(claims);
}

#[cfg(test)]
#[path = "raw_commit_tests.rs"]
mod raw_commit_tests;
