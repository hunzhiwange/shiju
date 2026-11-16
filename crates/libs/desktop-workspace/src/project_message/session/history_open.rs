//! Prepared history acceptance policy. Runtime rendering and permission effects stay with their owners.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistorySource {
    UiStore,
    Remote,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryDecision {
    Discard,
    LoadRemote,
    ReportRemoteError,
    KeepCurrent,
    RefreshMetadata,
    Replace,
}

pub struct HistoryOpen<'a> {
    pub active_session: Option<&'a str>,
    pub target_session: &'a str,
    pub current_generation: bool,
    pub source: HistorySource,
    pub current_ids: &'a [Option<String>],
    pub reset_boundary: Option<&'a [Option<String>]>,
    pub current_signature: u64,
}

impl HistoryOpen<'_> {
    pub fn accept(&self, prepared_session: Option<&str>) -> HistoryDecision {
        let active = self.active_session == Some(self.target_session) && self.current_generation;
        match prepared_session {
            Some(id) if active && id == self.target_session => HistoryDecision::Replace,
            Some(_) => HistoryDecision::Discard,
            None if !active => HistoryDecision::Discard,
            None if self.source == HistorySource::UiStore => HistoryDecision::LoadRemote,
            None => HistoryDecision::ReportRemoteError,
        }
    }

    pub fn content(
        &self,
        incoming_ids: &[Option<String>],
        incoming_signature: u64,
    ) -> HistoryDecision {
        if self.source == HistorySource::Remote {
            if super::lifecycle::remote_history_is_incomplete(self.current_ids, incoming_ids)
                || self.reset_boundary.is_some_and(|boundary| {
                    boundary == self.current_ids && incoming_ids != self.current_ids
                })
            {
                return HistoryDecision::KeepCurrent;
            }
            if self.current_signature == incoming_signature {
                return HistoryDecision::RefreshMetadata;
            }
        }
        HistoryDecision::Replace
    }
}
