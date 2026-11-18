//! Host-owned scheduling; page activity is an intent, never a client timer.
//!
//! Keep one schedule per authenticated scan owner for the gateway lifetime.
//! The host drives `tick` even when no desktop is connected and dispatches the
//! returned job on its blocking worker using `execute`. No missed ticks queue up.
use std::{collections::HashMap, time::{Duration, Instant, SystemTime}};

use super::{
    scan_jobs::{ScanAdapter, ScanJob, ScanJobError, ScanJobs, ScanOwner, ScanStep},
    source_settings::{UsageScanPreferences, UsageSourceSettings, scan_refresh_seconds},
};

pub const BACKGROUND_SCAN_INTERVAL: Duration = Duration::from_secs(600);

#[derive(Clone)]
pub struct ScanSchedule {
    jobs: ScanJobs,
    owner: ScanOwner,
    sources: HashMap<ScanAdapter, UsageSourceSettings>,
    paused: bool,
    preferences: Option<UsageScanPreferences>,
    page_active: bool,
    page_due: bool,
    last_background: Option<Instant>,
    last_page: Option<Instant>,
    manual: Option<ScanJob>,
}

impl ScanSchedule {
    pub fn new(jobs: ScanJobs, owner: ScanOwner) -> Self {
        Self {
            jobs, owner, sources: HashMap::new(), paused: false,
            preferences: None, page_active: false, page_due: false,
            last_background: None, last_page: None, manual: None,
        }
    }

    /// Supply preferences loaded by the authenticated host, never path grants.
    /// Unconfigured sources stay disabled until the host loads their settings.
    pub fn set_source(&mut self, adapter: ScanAdapter, settings: UsageSourceSettings) {
        if self.sources.get(&adapter) != Some(&settings) {
            self.sources.insert(adapter, settings);
            self.page_due = true;
        }
    }

    pub fn set_paused(&mut self, paused: bool) {
        if self.paused && !paused { self.page_due = true; }
        self.paused = paused;
    }

    /// Visibility recovery requests one immediate scan. Repeated heartbeats do
    /// not reset deadlines or create additional tasks.
    pub fn set_page_active(&mut self, active: bool) {
        if active && !self.page_active { self.page_due = true; }
        self.page_active = active;
    }

    pub fn set_preferences(&mut self, preferences: Option<UsageScanPreferences>) {
        if self.preferences.is_some_and(|old| !old.enabled)
            && preferences.is_none_or(|new| new.enabled)
        {
            self.page_due = true;
        }
        self.preferences = preferences;
    }

    fn page_interval(&self) -> Duration {
        let seconds = self.preferences.map_or_else(
            || scan_refresh_seconds(self.sources.values().cloned()),
            |preferences| preferences.refresh_seconds,
        );
        Duration::from_secs(u64::from(seconds))
    }

    /// Call at gateway startup and on a host timer. Monotonic time is injected
    /// so suspend/recovery and the ten-minute boundary need no real-time waits.
    /// Deadlines advance to now, skipping missed periods rather than replaying.
    pub fn tick(&mut self, now: Instant, has_enabled_source: bool) -> Result<Option<ScanJob>, ScanJobError> {
        if let Some(job) = self.manual.take() { return Ok(Some(job)); }
        if !has_enabled_source || self.paused || self.preferences.is_some_and(|p| !p.enabled) {
            return Ok(None);
        }
        let background_due = self.last_background.is_none_or(|last| {
            now.saturating_duration_since(last) >= BACKGROUND_SCAN_INTERVAL
        });
        let page_due = self.page_active && (self.page_due || self.last_page.is_none_or(|last| {
            now.saturating_duration_since(last) >= self.page_interval()
        }));
        if !background_due && !page_due { return Ok(None); }
        let job = self.jobs.start(&self.owner)?;
        self.last_background = Some(now);
        self.last_page = Some(now);
        self.page_due = false;
        Ok(Some(job))
    }

    /// Manual synchronization ignores automatic pause, while retaining source
    /// switches and the same single-flight job and complete-scan timestamp.
    pub fn manual_sync(&mut self, now: Instant) -> Result<ScanJob, ScanJobError> {
        let job = self.jobs.start(&self.owner)?;
        self.last_background = Some(now);
        self.last_page = Some(now);
        self.page_due = false;
        self.manual = Some(job.clone());
        Ok(job)
    }

    /// Host dispatch uses this gate for both manual and scheduled jobs. Disabled
    /// sources are skipped before the callback can discover or open any files.
    pub fn execute(
        &self,
        job: &ScanJob,
        mut step: impl FnMut(ScanAdapter) -> ScanStep,
    ) -> Result<ScanJob, ScanJobError> {
        self.jobs.execute(&self.owner, &job.task_id, |adapter| {
            if self.sources.get(&adapter).is_some_and(|source| source.scan_enabled) {
                step(adapter)
            } else {
                ScanStep::End
            }
        })
    }

    pub fn job(&self, id: &vw_api_transport::TaskId) -> Result<ScanJob, ScanJobError> {
        self.jobs.get(&self.owner, id)
    }

    pub fn cancel(&self, id: &vw_api_transport::TaskId) -> Result<ScanJob, ScanJobError> {
        self.jobs.cancel(&self.owner, id)
    }

    pub fn last_complete_scan(&self) -> Result<Option<SystemTime>, ScanJobError> {
        self.jobs.last_complete_scan(&self.owner)
    }
}

#[cfg(test)]
#[path = "scan_schedule_tests.rs"]
mod tests;
