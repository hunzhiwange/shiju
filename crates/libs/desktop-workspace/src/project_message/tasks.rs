//! Project background work; completion remains scoped to its originating request.
use desktop_file_tree::state::FindInFolderMatch;
use iced::Task;
pub struct SearchCompletion {
    pub tab_id: String,
    pub title: String,
    pub scope_path: String,
    pub query: String,
    pub replace_text: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub use_regex: bool,
    pub matches: Vec<FindInFolderMatch>,
    pub error: Option<String>,
    pub localized_error: Option<desktop_i18n::input::Feedback>,
    pub limit_reached: bool,
}
async fn spawn_blocking_opt<F, T>(f: F) -> Option<T>
where
    F: FnOnce() -> Option<T> + Send + 'static,
    T: Send + 'static,
{
    #[cfg(target_arch = "wasm32")]
    {
        f()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::task::spawn_blocking(f).await.unwrap_or(None)
    }
}
pub fn run_find_task(
    tab_id: String,
    title: String,
    scope_path: String,
    query: String,
    replace_text: String,
    case_sensitive: bool,
    whole_word: bool,
    use_regex: bool,
    files: Vec<String>,
) -> Task<SearchCompletion> {
    let tab_id_for_msg = tab_id.clone();
    let title_for_msg = title.clone();
    let scope_for_msg = scope_path.clone();
    let query_for_msg = query.clone();
    let replace_for_msg = replace_text.clone();

    Task::perform(
        async move {
            spawn_blocking_opt(move || {
                Some(super::search::search(
                    scope_path,
                    query,
                    case_sensitive,
                    whole_word,
                    use_regex,
                    files,
                ))
            })
            .await
            .unwrap_or((
                Vec::new(),
                Some(desktop_i18n::input::Feedback::new("搜索失败", vec![])),
                false,
            ))
        },
        move |(matches, error, limit_reached)| SearchCompletion {
            tab_id: tab_id_for_msg.clone(),
            title: title_for_msg.clone(),
            scope_path: scope_for_msg.clone(),
            query: query_for_msg.clone(),
            replace_text: replace_for_msg.clone(),
            case_sensitive,
            whole_word,
            use_regex,
            matches,
            error: error.as_ref().map(|feedback| feedback.fallback.clone()),
            localized_error: error,
            limit_reached,
        },
    )
}

pub async fn refresh_index(
    client: Result<vw_gateway_client::GatewayClient, String>,
    tenant: vw_api_project::TenantScopeDto,
    root: String,
    cache_directory: Option<std::path::PathBuf>,
) -> (super::data::FileTreeIndexSnapshot, Option<String>) {
    let remote = async {
        let client = client?;
        let context = super::gateway::resolve_gateway_file_context(&client, &tenant, &root).await?;
        let response = gateway_files::GatewayFilesClient::new(client.transport().clone())
            .file_list(&vw_api_file::file::ListFilesRequest {
                project_id: context.project_id,
                worktree_id: context.worktree_id,
                path: None,
                depth: None,
            })
            .await?;
        Ok::<_, String>(super::data::FileTreeIndexSnapshot::from_dto(
            &context.base_directory,
            &response.root,
        ))
    }
    .await;
    match remote {
        Ok(snapshot) => (snapshot, None),
        Err(error) => {
            let scan_root = root.clone();
            let files = spawn_blocking_opt(move || {
                Some(file_index::refresh_file_index(&scan_root, cache_directory.as_deref()))
            })
            .await
            .unwrap_or_default();
            let tree = desktop_file_tree::model::build_file_tree_model(&root, &files);
            (super::data::FileTreeIndexSnapshot { files, tree }, Some(error))
        }
    }
}

impl SearchCompletion {
    pub fn into_message<P, H, U>(self) -> super::message::ProjectMessage<P, H, U> {
        let SearchCompletion {
            tab_id,
            title,
            scope_path,
            query,
            replace_text,
            case_sensitive,
            whole_word,
            use_regex,
            matches,
            error,
            localized_error,
            limit_reached,
        } = self;
        super::message::ProjectMessage::FileTreeFindCompleted {
            tab_id,
            title,
            scope_path,
            query,
            replace_text,
            case_sensitive,
            whole_word,
            use_regex,
            matches,
            error,
            localized_error,
            limit_reached,
        }
    }
}
