//! 复用文件管理器的搜索交互，通过网关搜索当前检出目录。
use super::{Workbench, files, message};
use crate::app::Message;
use desktop_file_tree::{ProjectMessage, state::FindInFolderMatch};
use desktop_i18n::input::Feedback;
use desktop_workspace::project_message::{
    find::{self, FindEffect, FindMessage},
    tasks::SearchCompletion,
};
use iced::Task;

pub(super) fn update(state: &mut Workbench, event: ProjectMessage) -> Task<Message> {
    let event = desktop_file_tree::Message::Project(event);
    let Message::Project(event) = crate::app::components::file_tree::map_message(event) else {
        return Task::none();
    };
    let Ok(event) = FindMessage::try_from(event) else {
        return Task::none();
    };
    if let FindMessage::FileTreeFindRun(id) = &event
        && state.files.find.find_results_tabs.iter().any(|tab| &tab.id == id && tab.running)
    {
        return Task::none();
    }
    let root = state.files.root.clone();
    let effects = find::handle(&mut state.files.find, root.as_deref(), Vec::new(), event);
    Task::batch(effects.into_iter().filter_map(|effect| match effect {
        FindEffect::Preview { path, line, column } => {
            let relative = files::relative_path(root.as_deref()?, &path)?;
            Some(files::open_file(state, relative, Some((line, column))))
        }
        FindEffect::Search {
            tab_id,
            title,
            scope_path,
            query,
            replace_text,
            case_sensitive,
            whole_word,
            use_regex,
            ..
        } => {
            let root = root.as_deref()?.to_owned();
            Some(search(
                state,
                root,
                SearchCompletion {
                    tab_id,
                    title,
                    scope_path,
                    query,
                    replace_text,
                    case_sensitive,
                    whole_word,
                    use_regex,
                    matches: Vec::new(),
                    error: None,
                    localized_error: None,
                    limit_reached: false,
                },
            ))
        }
        FindEffect::None
        | FindEffect::ShowFileManager
        | FindEffect::Open { .. }
        | FindEffect::Close { .. } => None,
    }))
}

fn search(state: &Workbench, root: String, mut completion: SearchCompletion) -> Task<Message> {
    let client = state.client();
    let epoch = state.epoch;
    Task::perform(
        async move {
            match search_matches(client, &root, &completion).await {
                Ok((matches, limit_reached)) => {
                    completion.matches = matches;
                    completion.limit_reached = limit_reached;
                }
                Err(feedback) => {
                    completion.error = Some(feedback.fallback.clone());
                    completion.localized_error = Some(feedback);
                }
            }
            completion
        },
        move |result| {
            message(super::Event::Files(epoch, files::Event::SearchReady(Box::new(result))))
        },
    )
}

async fn search_matches(
    client: Result<vw_gateway_client::GatewayClient, String>,
    root: &str,
    completion: &SearchCompletion,
) -> Result<(Vec<FindInFolderMatch>, bool), Feedback> {
    const MAX_RESULTS: usize = 3000;
    let scope = files::relative_path(root, &completion.scope_path)
        .ok_or_else(|| Feedback::new("作用域必须是项目内的相对路径", vec![]))?;
    let mut pattern = if completion.use_regex {
        completion.query.clone()
    } else {
        regex::escape(&completion.query)
    };
    if completion.whole_word {
        pattern = format!(r"\b(?:{pattern})\b");
    }
    if !completion.case_sensitive {
        pattern = format!("(?i:{pattern})");
    }
    regex::Regex::new(&pattern)
        .map_err(|error| Feedback::new("正则表达式错误: %s", vec![error.to_string().into()]))?;
    let error = |error: String| Feedback::new("搜索失败: %s", vec![error.into()]);
    let client = client.map_err(error)?;
    let results = gateway_files::GatewayFilesClient::new(client.transport().clone())
        .search_directory_text(root, &scope, pattern)
        .await
        .map_err(error)?;
    let limit_reached = results.len() > MAX_RESULTS;
    let matches = results
        .into_iter()
        .take(MAX_RESULTS)
        .map(|item| FindInFolderMatch {
            path: files::absolute_path(root, &item.path),
            line: item.line_number,
            column: item.start + 1,
            preview: item.line,
            match_len: item.end.saturating_sub(item.start),
        })
        .collect();
    Ok((matches, limit_reached))
}

pub(super) fn complete(state: &mut Workbench, completion: SearchCompletion) {
    let find = &mut state.files.find;
    if !find.find_results_tabs.iter().any(|tab| tab.id == completion.tab_id && tab.running) {
        return;
    }
    let active = find.active_find_results_tab_id.clone();
    let event = FindMessage::try_from(completion.into_message::<(), (), ()>())
        .expect("search completion maps to a find event");
    let _ = find::handle(find, state.files.root.as_deref(), Vec::new(), event);
    find.active_find_results_tab_id = active;
}
