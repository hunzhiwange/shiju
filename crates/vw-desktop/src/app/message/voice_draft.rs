//! Generation and draft ownership checks shared by the composer and its tests.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Scope {
    pub endpoint: String,
    pub tenant: String,
    pub project: Option<String>,
    pub session: String,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Phase {
    #[default]
    Idle,
    Recording,
    Transcribing,
    Error,
}
#[derive(Default)]
pub(crate) struct Draft {
    pub phase: Phase,
    pub generation: u64,
    pub scope: Option<Scope>,
    original: String,
    edited: bool,
    pub pending: Option<String>,
}
impl Draft {
    pub fn start(&mut self, scope: Scope, text: String) -> Option<u64> {
        if matches!(self.phase, Phase::Recording | Phase::Transcribing) || self.pending.is_some() {
            return None;
        }
        self.generation = self.generation.checked_add(1)?;
        self.scope = Some(scope);
        self.original = text;
        self.edited = false;
        self.phase = Phase::Recording;
        Some(self.generation)
    }
    pub fn observe(&mut self, current: &str) {
        if self.original != current {
            self.edited();
        }
    }
    pub fn edited(&mut self) {
        self.edited = true;
    }
    pub fn cancel(&mut self) {
        self.scope = None;
        self.pending = None;
        self.phase = Phase::Idle;
    }
    pub fn complete(
        &mut self,
        generation: u64,
        scope: &Scope,
        current: &str,
        text: String,
    ) -> Option<String> {
        if self.phase != Phase::Transcribing
            || self.generation != generation
            || self.scope.as_ref() != Some(scope)
        {
            return None;
        }
        self.phase = Phase::Idle;
        if text.trim().is_empty() {
            return None;
        }
        if self.edited || self.original != current {
            self.pending = Some(text);
            return None;
        }
        self.scope = None;
        Some(text)
    }
}
