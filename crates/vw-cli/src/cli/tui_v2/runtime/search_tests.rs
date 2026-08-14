use std::path::Path;

use super::super::effect::TuiEffect;
use super::super::model::search::{SearchMode, SearchResult, SearchState};
use super::search::{SearchProvider, run_search, search_dialog_lines, validated_open_effect};

struct ReversedProvider;

impl SearchProvider for ReversedProvider {
    fn search(&mut self, query: &str, generation: u64) -> Result<Vec<SearchResult>, String> {
        Ok(vec![SearchResult {
            id: generation.to_string(),
            label: format!("{query}.rs"),
            preview: format!("generation {generation}"),
            line: Some(3),
        }])
    }
}

#[test]
fn tui_search_dialogs_discard_stale_results_and_open_valid_targets() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::create_dir(workspace.path().join("src")).unwrap();
    std::fs::write(workspace.path().join("src/main.rs"), "fn main() {}\n").unwrap();
    std::fs::write(workspace.path().join(".env"), "SECRET=hidden\n").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("/tmp", workspace.path().join("outside")).unwrap();

    let mut state = SearchState::new(SearchMode::QuickOpen, "prompt");
    let first = state.begin_request("old");
    let second = state.begin_request("main");
    assert!(!state.apply_results(
        first,
        Ok(vec![SearchResult {
            id: "old".to_string(),
            label: "old.rs".to_string(),
            preview: String::new(),
            line: None,
        }]),
    ));
    assert!(state.apply_results(
        second,
        Ok(vec![SearchResult {
            id: "new".to_string(),
            label: "src/main.rs".to_string(),
            preview: "fn main".to_string(),
            line: Some(1),
        }]),
    ));
    assert_eq!(state.results[0].id, "new");

    let effect =
        validated_open_effect(workspace.path(), Path::new("src/main.rs"), Some(1)).unwrap();
    let TuiEffect::OpenWorkspaceFile { path, line } = effect else {
        panic!("expected open effect");
    };
    assert!(path.starts_with(workspace.path().canonicalize().unwrap()));
    assert_eq!(line, Some(1));
    assert!(validated_open_effect(workspace.path(), Path::new("../outside"), None).is_err());
    assert!(validated_open_effect(workspace.path(), Path::new(".env"), None).is_err());
    #[cfg(unix)]
    assert!(validated_open_effect(workspace.path(), Path::new("outside/file"), None).is_err());

    let focus = state.cancel();
    assert_eq!(focus, "prompt");
    assert!(!state.apply_results(second, Ok(Vec::new())), "closed dialogs ignore late results");

    let mut provider = ReversedProvider;
    let mut live = SearchState::new(SearchMode::Global, "transcript");
    run_search(&mut live, "needle", &mut provider);
    assert_eq!(live.results[0].label, "needle.rs");
}

#[test]
fn tui_parity_golden_search_dialogs() {
    let mut state = SearchState::new(SearchMode::QuickOpen, "prompt");
    state.query = "main".to_string();
    state.results = vec![SearchResult {
        id: "src/main.rs".to_string(),
        label: "src/main.rs".to_string(),
        preview: "fn main()".to_string(),
        line: Some(1),
    }];
    assert_eq!(
        search_dialog_lines(&state, 40).join("\n"),
        "QuickOpen /main\n> src/main.rs · fn main()"
    );
}
