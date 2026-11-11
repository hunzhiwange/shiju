use super::{ModelsRefresh, ModelsRefreshStatus, ModelsSnapshot, Provider};
use super::{bundled_models, merge_bundled_protocols, merge_bundled_shiju, parse_models_text};
use crate::installation;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

pub(super) const STALE_TIME: Duration = Duration::from_secs(60 * 60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_RESPONSE_BYTES: usize = 32 * 1024 * 1024;

pub(super) struct ModelsCache {
    path: PathBuf,
    custom_path: Option<PathBuf>,
    api_url: String,
    refresh_gate: Arc<tokio::sync::Mutex<()>>,
    state: Mutex<CacheState>,
}

#[derive(Default)]
struct CacheState {
    snapshot: Option<Arc<ModelsSnapshot>>,
    snapshot_revision: u64,
    completed_refreshes: u64,
    last_refresh: Option<(Instant, ModelsRefresh)>,
}

impl ModelsCache {
    pub(super) fn new(path: PathBuf, custom_path: Option<PathBuf>, api_url: String) -> Self {
        Self { path, custom_path, api_url, refresh_gate: Arc::default(), state: Mutex::default() }
    }

    pub(super) fn invalidate(&self) {
        let mut state = self.state.lock().unwrap();
        state.snapshot = None;
        state.snapshot_revision += 1;
    }

    pub(super) async fn snapshot(self: &Arc<Self>) -> Arc<ModelsSnapshot> {
        loop {
            let revision = {
                let state = self.state.lock().unwrap();
                if let Some(snapshot) = &state.snapshot {
                    return Arc::clone(snapshot);
                }
                state.snapshot_revision
            };
            let cache = Arc::clone(self);
            let snapshot = tokio::task::spawn_blocking(move || {
                let path = cache.custom_path.as_ref().unwrap_or(&cache.path);
                read_snapshot(path, cache.custom_path.is_some()).unwrap_or_else(|| ModelsSnapshot {
                    providers: bundled_models("load_bundled"),
                    fetched_at: None,
                })
            })
            .await
            .expect("model snapshot reader panicked");

            let mut state = self.state.lock().unwrap();
            // A refresh or explicit invalidation may have superseded the disk read.
            if state.snapshot_revision == revision {
                return Arc::clone(state.snapshot.get_or_insert_with(|| Arc::new(snapshot)));
            }
        }
    }

    pub(super) async fn refresh(self: &Arc<Self>, disabled: bool) -> ModelsRefresh {
        if disabled {
            return ModelsRefresh {
                status: ModelsRefreshStatus::Disabled,
                fetched_at: self.remote_fetched_at().await,
                last_error: None,
            };
        }

        let observed_refreshes = self.state.lock().unwrap().completed_refreshes;
        let gate = Arc::clone(&self.refresh_gate).lock_owned().await;
        {
            let state = self.state.lock().unwrap();
            if let Some((attempted_at, result)) = &state.last_refresh {
                if observed_refreshes != state.completed_refreshes {
                    return result.clone();
                }
                if result.status == ModelsRefreshStatus::Updated
                    && attempted_at.elapsed() < STALE_TIME
                {
                    return ModelsRefresh { status: ModelsRefreshStatus::Fresh, ..result.clone() };
                }
            }
        }

        let body = match download(&self.api_url).await {
            Ok(body) => body,
            Err(error) => {
                let result = failed(self.remote_fetched_at().await, error);
                self.record_refresh(&result, None);
                return result;
            }
        };

        let cache = Arc::clone(self);
        tokio::task::spawn_blocking(move || {
            // Once the atomic commit starts, keep the gate even if its caller is cancelled.
            let _gate = gate;
            let (result, snapshot) = match install_snapshot(&cache.path, &body) {
                Ok(snapshot) => (
                    ModelsRefresh {
                        status: ModelsRefreshStatus::Updated,
                        fetched_at: snapshot.fetched_at,
                        last_error: None,
                    },
                    Some(snapshot),
                ),
                Err(error) => {
                    let fetched_at =
                        read_snapshot(&cache.path, false).and_then(|value| value.fetched_at);
                    (failed(fetched_at, error), None)
                }
            };
            cache.record_refresh(&result, snapshot);
            result
        })
        .await
        .expect("model snapshot writer panicked")
    }

    async fn remote_fetched_at(&self) -> Option<SystemTime> {
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || read_snapshot(&path, false)?.fetched_at)
            .await
            .expect("model snapshot reader panicked")
    }

    fn record_refresh(&self, result: &ModelsRefresh, snapshot: Option<ModelsSnapshot>) {
        let mut state = self.state.lock().unwrap();
        if let Some(snapshot) = snapshot {
            state.snapshot_revision += 1;
            state.snapshot = self.custom_path.is_none().then(|| Arc::new(snapshot));
        }
        state.completed_refreshes += 1;
        state.last_refresh = Some((Instant::now(), result.clone()));
    }
}

fn failed(fetched_at: Option<SystemTime>, error: String) -> ModelsRefresh {
    tracing::warn!(%error, "models.dev refresh failed");
    ModelsRefresh { status: ModelsRefreshStatus::Failed, fetched_at, last_error: Some(error) }
}

fn read_snapshot(path: &Path, custom: bool) -> Option<ModelsSnapshot> {
    let mut file = File::open(path).ok()?;
    let mut text = String::new();
    file.read_to_string(&mut text).ok()?;
    let mut providers = parse_models_text("load_cache", &text)?;
    if providers.is_empty() || (!custom && !usable_catalog(&providers)) {
        return None;
    }
    let fetched_at = if custom {
        None
    } else {
        // Read metadata from the same inode as the JSON, even if the path was replaced.
        file.metadata().and_then(|metadata| metadata.modified()).ok()
    };
    merge_bundled_shiju(&mut providers, "load_cache");
    merge_bundled_protocols(&mut providers);
    Some(ModelsSnapshot { providers, fetched_at })
}

fn usable_catalog(providers: &HashMap<String, Provider>) -> bool {
    providers.values().any(|provider| !provider.models.is_empty())
        && providers.iter().all(|(key, provider)| {
            !key.trim().is_empty()
                && !provider.id.trim().is_empty()
                && provider
                    .models
                    .iter()
                    .all(|(key, model)| !key.trim().is_empty() && !model.id.trim().is_empty())
        })
}

fn install_snapshot(path: &Path, body: &[u8]) -> Result<ModelsSnapshot, String> {
    let text = std::str::from_utf8(body).map_err(|_| "models.dev response is not UTF-8")?;
    let mut providers = parse_models_text("refresh_cache", text)
        .filter(usable_catalog)
        .ok_or("models.dev response does not contain a usable model catalog")?;
    merge_bundled_shiju(&mut providers, "refresh_cache");
    merge_bundled_protocols(&mut providers);
    let fetched_at = replace_snapshot(path, body)
        .map_err(|error| format!("Could not replace models.json ({})", error.kind()))?;
    Ok(ModelsSnapshot { providers, fetched_at: Some(fetched_at) })
}

fn replace_snapshot(path: &Path, body: &[u8]) -> io::Result<SystemTime> {
    let parent = path.parent().ok_or(io::ErrorKind::InvalidInput)?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)?;
    staged.write_all(body)?;
    staged.as_file().sync_all()?;
    let fetched_at = staged.as_file().metadata()?.modified()?;
    staged.persist(path)?;
    Ok(fetched_at)
}

async fn download(api_url: &str) -> Result<Vec<u8>, String> {
    // A configured URL can contain credentials. Keep redirects on its original origin.
    let client = reqwest::Client::builder()
        .referer(false)
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            let destination = attempt.url();
            if attempt.previous().len() >= 10
                || attempt
                    .previous()
                    .first()
                    .is_none_or(|initial| initial.origin() != destination.origin())
                || !destination.username().is_empty()
                || destination.password().is_some()
            {
                return attempt.stop();
            }
            attempt.follow()
        }))
        .build()
        .map_err(|_| "Could not initialize models.dev HTTP client")?;
    let mut response = client
        .get(api_url)
        .header(reqwest::header::USER_AGENT, installation::user_agent())
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(download_error)?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("models.dev returned HTTP {status}"));
    }

    let too_large = || format!("models.dev response exceeds {MAX_RESPONSE_BYTES} bytes");
    if response.content_length().is_some_and(|len| len > MAX_RESPONSE_BYTES as u64) {
        return Err(too_large());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(download_error)? {
        if chunk.len() > MAX_RESPONSE_BYTES - bytes.len() {
            return Err(too_large());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn download_error(error: reqwest::Error) -> String {
    if error.is_timeout() {
        "models.dev request timed out".into()
    } else {
        "Could not download models.dev response".into()
    }
}
