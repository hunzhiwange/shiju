use desktop_chat_input::input_panel::suggestions::State as Suggestions;
use desktop_session::message::chat::input::file_search::FileSearchResult;
struct Fixture {
    workspace_state: desktop_workspace::project_state::State<()>,
    input_suggestions: Suggestions,
    project_path: Option<String>,
    search_text: String,
}
fn new_app() -> Fixture {
    Fixture {
        workspace_state: desktop_workspace::project_state::State::new(
            &serde_json::json!({}),
            vec![],
            vec![],
            Default::default(),
        ),
        input_suggestions: Suggestions::default(),
        project_path: None,
        search_text: String::new(),
    }
}
impl Fixture {
    fn current_file_index(&self) -> &[String] {
        self.workspace_state.current_file_index(self.project_path.as_deref())
    }
    fn has_file_index(&self, path: &str) -> bool {
        self.workspace_state.file_index_cache.contains_key(path)
    }
    fn refresh_search_panel_file_cache(&mut self) {
        self.workspace_state.refresh_search_panel(
            self.project_path.as_deref(),
            desktop_session::message::chat::input::file_search::normalize_file_search_query(
                &self.search_text,
            ),
        );
    }
    fn refresh_file_search_cache(&mut self) {
        self.input_suggestions.refresh_file_cache(
            self.workspace_state.current_file_index(self.project_path.as_deref()),
            self.project_path.as_deref(),
            self.workspace_state.file_index_revision,
        );
    }
    fn set_file_index(&mut self, path: &str, files: Vec<String>) {
        self.workspace_state.set_file_index(path, files);
        self.refresh_search_panel_file_cache();
        self.refresh_file_search_cache();
    }
    fn cached_search_panel_file_results(&self) -> &[String] {
        &self.workspace_state.search_panel_file_cache_results
    }
    fn cached_file_search_entries(
        &self,
    ) -> &[desktop_session::message::chat::input::file_search::FileSearchResult] {
        &self.input_suggestions.file_search_cache_entries
    }
}
#[test]
fn file_index_cache_tracks_project_results_and_tree_model() {
    let mut app = new_app();
    app.project_path = Some("/workspace".to_string());
    app.search_text = " SRC\\LIB ".to_string();
    app.input_suggestions.file_search_query = "src".to_string();

    app.set_file_index(
        "/workspace",
        vec![
            "src/lib.rs".to_string(),
            "src/main.rs".to_string(),
            "README.md".to_string(),
            "tests/app_basic.rs".to_string(),
        ],
    );

    assert!(app.has_file_index("/workspace"));
    assert_eq!(
        app.current_file_index(),
        ["src/lib.rs", "src/main.rs", "README.md", "tests/app_basic.rs"]
    );
    assert!(
        app.project_path
            .as_ref()
            .is_some_and(|path| app.workspace_state.file_tree_model_cache.contains_key(path))
    );
    assert_eq!(app.cached_search_panel_file_results(), ["src/lib.rs"]);
    assert!(app.cached_file_search_entries().iter().any(|entry| entry.path == "src/"));
    assert!(app.cached_file_search_entries().iter().any(|entry| entry.path == "src/lib.rs"));

    let cached_results = app.cached_search_panel_file_results().to_vec();
    app.refresh_search_panel_file_cache();
    assert_eq!(app.cached_search_panel_file_results(), cached_results);

    app.project_path = Some("/other".to_string());
    assert!(app.current_file_index().is_empty());
    assert!(
        app.project_path
            .as_ref()
            .is_none_or(|path| !app.workspace_state.file_tree_model_cache.contains_key(path))
    );
}

#[test]
fn search_panel_file_cache_limits_to_first_eight_matches() {
    let mut app = new_app();
    app.project_path = Some("/workspace".to_string());
    app.search_text = "src".to_string();
    app.set_file_index(
        "/workspace",
        (0..10).map(|idx| format!("src/file_{idx}.rs")).collect::<Vec<_>>(),
    );

    assert_eq!(app.cached_search_panel_file_results().len(), 8);
    assert_eq!(app.cached_search_panel_file_results()[0], "src/file_0.rs");
    assert_eq!(app.cached_search_panel_file_results()[7], "src/file_7.rs");
}

#[test]
fn search_caches_keep_entries_when_cache_keys_match() {
    let mut app = new_app();
    app.project_path = Some("/workspace".to_string());
    app.workspace_state.file_index_revision = 9;

    app.search_text = " SRC\\ ".to_string();
    app.workspace_state.search_panel_file_cache_query = "src/".to_string();
    app.workspace_state.search_panel_file_cache_project_path = Some("/workspace".to_string());
    app.workspace_state.search_panel_file_cache_revision = 9;
    app.workspace_state.search_panel_file_cache_results = vec!["kept-panel-result".to_string()];

    app.input_suggestions.file_search_query = " SRC\\ ".to_string();
    app.input_suggestions.file_search_cache_query = "src/".to_string();
    app.input_suggestions.file_search_cache_project_path = Some("/workspace".to_string());
    app.input_suggestions.file_search_cache_revision = 9;
    app.input_suggestions.file_search_cache_entries =
        vec![FileSearchResult { path: "kept-file-result".to_string(), is_dir: true }];

    app.refresh_search_panel_file_cache();
    app.refresh_file_search_cache();

    assert_eq!(app.cached_search_panel_file_results(), ["kept-panel-result"]);
    assert_eq!(app.cached_file_search_entries().len(), 1);
    assert_eq!(app.cached_file_search_entries()[0].path, "kept-file-result");
    assert!(app.cached_file_search_entries()[0].is_dir);
}

#[test]
fn search_caches_invalidate_for_query_project_and_revision_changes() {
    let mut app = new_app();
    app.workspace_state.file_index_cache.clear();
    app.workspace_state
        .file_index_cache
        .insert("/workspace".to_string(), vec!["src/lib.rs".to_string(), "README.md".to_string()]);
    app.workspace_state
        .file_index_cache
        .insert("/other".to_string(), vec!["src/library.rs".to_string()]);
    app.project_path = Some("/workspace".to_string());
    app.workspace_state.file_index_revision = 10;
    app.search_text = " SRC\\LIB ".to_string();
    app.input_suggestions.file_search_query = String::new();

    app.refresh_search_panel_file_cache();
    app.refresh_file_search_cache();

    assert_eq!(app.workspace_state.search_panel_file_cache_query, "src/lib");
    assert_eq!(app.cached_search_panel_file_results(), ["src/lib.rs"]);
    assert!(app.cached_file_search_entries().iter().any(|entry| entry.path == "src/lib.rs"));

    app.project_path = Some("/other".to_string());
    app.refresh_search_panel_file_cache();
    app.refresh_file_search_cache();

    assert_eq!(app.cached_search_panel_file_results(), ["src/library.rs"]);
    assert!(app.cached_file_search_entries().iter().any(|entry| entry.path == "src/library.rs"));
    assert!(!app.cached_file_search_entries().iter().any(|entry| entry.path == "src/lib.rs"));

    app.workspace_state
        .file_index_cache
        .insert("/other".to_string(), vec!["src/lib_new.rs".to_string()]);
    app.workspace_state.file_index_revision = 11;
    app.refresh_search_panel_file_cache();
    app.refresh_file_search_cache();

    assert_eq!(app.workspace_state.search_panel_file_cache_revision, 11);
    assert_eq!(app.cached_search_panel_file_results(), ["src/lib_new.rs"]);
    assert_eq!(app.input_suggestions.file_search_cache_revision, 11);
    assert!(app.cached_file_search_entries().iter().any(|entry| entry.path == "src/lib_new.rs"));
    assert!(!app.cached_file_search_entries().iter().any(|entry| entry.path == "src/library.rs"));
}

#[test]
fn set_file_index_overwrites_existing_files_and_wraps_revision() {
    let mut app = new_app();
    app.workspace_state.file_index_cache.clear();
    app.workspace_state.file_tree_model_cache.clear();
    app.workspace_state
        .file_index_cache
        .insert("/workspace".to_string(), vec!["old.rs".to_string()]);
    app.project_path = Some("/workspace".to_string());
    app.search_text = "new".to_string();
    app.input_suggestions.file_search_query = "new".to_string();
    app.workspace_state.file_index_revision = u64::MAX;

    app.set_file_index("/workspace", vec!["src/new.rs".to_string()]);

    assert_eq!(app.workspace_state.file_index_revision, 0);
    assert_eq!(app.current_file_index(), ["src/new.rs"]);
    assert!(
        app.project_path
            .as_ref()
            .is_some_and(|path| app.workspace_state.file_tree_model_cache.contains_key(path))
    );
    assert_eq!(app.workspace_state.search_panel_file_cache_revision, 0);
    assert_eq!(app.cached_search_panel_file_results(), ["src/new.rs"]);
    assert_eq!(app.input_suggestions.file_search_cache_revision, 0);
    assert!(app.cached_file_search_entries().iter().any(|entry| entry.path == "src/new.rs"));
    assert!(!app.cached_file_search_entries().iter().any(|entry| entry.path == "old.rs"));
}
