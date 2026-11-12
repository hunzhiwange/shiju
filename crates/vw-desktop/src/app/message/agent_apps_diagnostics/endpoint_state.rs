//! Pending endpoint choice and host-owned diagnostic evidence; no provider configuration copy.
use super::model_state::ModelContext;
use crate::app::state::app_providers::edit::{EditForm, Mode};
use vw_gateway_client::{GatewayClient, vw_api_types::agent_apps_diagnostics::*};
#[derive(Clone)]
pub(crate) struct Transport {
    pub client: GatewayClient,
    pub token: String,
}
impl std::fmt::Debug for Transport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EndpointTransport([redacted])")
    }
}
#[derive(Clone, Debug)]
pub(crate) struct State {
    pub context: ModelContext,
    pub generation: u64,
    pub transport: Transport,
    pub candidates: Vec<DiagnosticEndpointCandidate>,
    pub selected: Option<DiagnosticEndpointId>,
    pub preset_name: String,
    pub loading: bool,
    pub starting: bool,
    pub cancelling: bool,
    pub closing: bool,
    pub serial: u64,
    pub job: Option<DiagnosticJobIdentity>,
    pub snapshot: Option<DiagnosticJobSnapshot>,
    pub error: Option<String>,
}
impl State {
    pub fn terminal(&self) -> bool {
        self.snapshot.as_ref().is_some_and(|s| {
            matches!(
                s.state,
                DiagnosticJobState::Succeeded {}
                    | DiagnosticJobState::Failed { .. }
                    | DiagnosticJobState::Cancelled {}
                    | DiagnosticJobState::Interrupted {}
            )
        })
    }
    pub fn active(&self) -> bool {
        self.starting || (self.job.is_some() && !self.terminal())
    }
    pub fn report(&self, id: &DiagnosticEndpointId) -> Option<&DiagnosticReport<SpeedEvidence>> {
        self.snapshot.as_ref()?.results.as_ref()?.iter().find_map(|r| match r {
            DiagnosticResult::Speed { endpoint_id, report } if endpoint_id == id => Some(report),
            _ => None,
        })
    }
    pub fn sorted(&self) -> Vec<&DiagnosticEndpointCandidate> {
        let mut rows: Vec<_> = self.candidates.iter().collect();
        rows.sort_by_key(|r| match self.report(&r.id).map(|r| &r.outcome) {
            Some(DiagnosticOutcome::Succeeded { evidence }) => (0, evidence.latency_ms),
            None => (1, 0),
            _ => (2, 0),
        });
        rows
    }
    pub fn accept(&mut self, serial: u64, snapshot: DiagnosticJobSnapshot) -> bool {
        if self.serial != serial || self.terminal() || self.job.as_ref() != Some(&snapshot.job)
            || !self.context.matches(&snapshot.job, DiagnosticProbeKind::Speed)
            || !snapshot.results.iter().flatten().all(|r| matches!(r,
                DiagnosticResult::Speed { endpoint_id, .. } if self.candidates.iter().any(|c| &c.id == endpoint_id))) {
            return false;
        }
        self.cancelling = matches!(snapshot.state, DiagnosticJobState::Cancelling {});
        self.snapshot = Some(snapshot);
        true
    }
    pub fn choose(&mut self, id: DiagnosticEndpointId) {
        if self.candidates.iter().any(|c| c.id == id) {
            self.selected = Some(id);
        }
    }
    pub fn apply(&self, form: &mut EditForm) -> Result<(), &'static str> {
        if self.active()
            || form.saving
            || form.id != self.context.provider.as_ref()
            || !editable(form)
        {
            return Err("请等待测速结束并确认当前供应商");
        }
        let url = &self
            .candidates
            .iter()
            .find(|c| Some(&c.id) == self.selected.as_ref())
            .ok_or("请选择候选端点")?
            .display_url;
        if let Some(draft) = &mut form.claude {
            draft.base_url = url.clone();
        } else if let Some(draft) = &mut form.gemini {
            draft.base_url = url.clone();
        } else if let Some(draft) = &mut form.codex {
            let mut doc: toml_edit::DocumentMut =
                draft.toml.text().parse().map_err(|_| "Codex TOML 无效")?;
            let selected = codex_selector(&doc)?;
            let slot = match selected {
                Some(name) => &mut doc["model_providers"][&name],
                None => doc.as_item_mut(),
            };
            let key = if slot.get("openai_base_url").is_some() && slot.get("base_url").is_none() {
                "openai_base_url"
            } else {
                "base_url"
            };
            let mut value = toml_edit::Value::from(url.clone());
            if let Some(previous) = slot.get(key).and_then(toml_edit::Item::as_value) {
                *value.decor_mut() = previous.decor().clone();
            }
            slot[key] = toml_edit::Item::Value(value);
            draft.toml = iced::widget::text_editor::Content::with_text(&doc.to_string());
        }
        Ok(())
    }
}
pub(crate) fn editable(form: &EditForm) -> bool {
    matches!(form.mode, Mode::Update)
        && !form.saving
        && (form
            .claude
            .as_ref()
            .is_some_and(|d| d.kind == crate::app::state::app_providers::claude::Kind::ThirdParty)
            || form.gemini.as_ref().is_some_and(|d| {
                d.kind == crate::app::state::app_providers::gemini::Kind::ThirdParty
            })
            || form.codex.is_some())
}

fn codex_selector(doc: &toml_edit::DocumentMut) -> Result<Option<String>, &'static str> {
    let selector = doc.get("model_provider").and_then(toml_edit::Item::as_str).unwrap_or("").trim();
    if !selector.is_empty()
        && let Some(table) = doc.get("model_providers").and_then(|p| p.get(selector))
    {
        if table.as_table_like().is_none() {
            return Err("Codex 供应商配置无效");
        }
        return Ok(Some(selector.to_owned()));
    }
    if !matches!(selector, "" | "openai") {
        return Err("Codex 当前供应商配置缺失");
    }
    Ok(None)
}
pub(crate) fn query(
    form: &EditForm,
    preset: &str,
) -> Result<DiagnosticEndpointCandidatesQuery, &'static str> {
    let url = if let Some(d) = &form.claude {
        d.base_url.clone()
    } else if let Some(d) = &form.gemini {
        d.base_url.clone()
    } else if let Some(d) = &form.codex {
        let doc: toml_edit::DocumentMut = d.toml.text().parse().map_err(|_| "Codex TOML 无效")?;
        let selected = codex_selector(&doc)?;
        let slot = match selected.as_ref() {
            Some(name) => &doc["model_providers"][name],
            None => doc.as_item(),
        };
        slot.get("base_url")
            .or_else(|| slot.get("openai_base_url"))
            .and_then(toml_edit::Item::as_str)
            .unwrap_or("")
            .to_owned()
    } else {
        return Err("当前供应商不支持端点测速");
    };
    Ok(DiagnosticEndpointCandidatesQuery {
        provider_id: form.id.clone().into(),
        draft_urls: if url.trim().is_empty() { vec![] } else { vec![url] },
        preset_name: (!preset.trim().is_empty()).then(|| preset.trim().to_owned()),
    })
}
