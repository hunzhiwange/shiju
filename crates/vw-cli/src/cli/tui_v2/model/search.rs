#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchMode {
    QuickOpen,
    Global,
    History,
    Transcript,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SearchResult {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) preview: String,
    pub(crate) line: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SearchState {
    pub(crate) mode: SearchMode,
    pub(crate) query: String,
    pub(crate) generation: u64,
    pub(crate) results: Vec<SearchResult>,
    pub(crate) selected: usize,
    pub(crate) active: bool,
    pub(crate) original_focus: String,
    pub(crate) error: Option<String>,
}

impl SearchState {
    pub(crate) fn new(mode: SearchMode, original_focus: impl Into<String>) -> Self {
        Self {
            mode,
            query: String::new(),
            generation: 0,
            results: Vec::new(),
            selected: 0,
            active: true,
            original_focus: original_focus.into(),
            error: None,
        }
    }

    pub(crate) fn begin_request(&mut self, query: impl Into<String>) -> u64 {
        self.query = query.into();
        self.generation = self.generation.saturating_add(1);
        self.error = None;
        self.generation
    }

    pub(crate) fn apply_results(
        &mut self,
        generation: u64,
        result: Result<Vec<SearchResult>, String>,
    ) -> bool {
        if !self.active || generation != self.generation {
            return false;
        }
        match result {
            Ok(results) => {
                self.results = results;
                self.selected = self.selected.min(self.results.len().saturating_sub(1));
            }
            Err(error) => self.error = Some(error),
        }
        true
    }

    pub(crate) fn cancel(&mut self) -> String {
        self.active = false;
        self.original_focus.clone()
    }
}
