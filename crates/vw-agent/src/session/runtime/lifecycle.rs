//! 原生会话创建、加载和归属校验。

use std::path::PathBuf;

use crate::project;
use crate::session::{message, session, ui_store};
use crate::storage;

use super::types::{CreateSessionInput, LifecycleError, SessionScope, SessionSnapshot};

fn canonical_recorded_directory(directory: &str) -> PathBuf {
    let path = PathBuf::from(directory);
    if !path.is_absolute() {
        return path;
    }
    std::fs::canonicalize(&path).unwrap_or(path)
}

fn session_not_found(error: session::Error, session_id: &str) -> LifecycleError {
    match error {
        session::Error::Storage(storage::Error::NotFound(_)) => {
            LifecycleError::SessionNotFound { session_id: session_id.to_string() }
        }
        error => LifecycleError::Session(error),
    }
}

async fn load_info_for_scope(
    scope: &SessionScope,
    expected_project_id: &str,
    session_id: &str,
) -> Result<session::Info, LifecycleError> {
    let info =
        session::get_any(session_id).await.map_err(|error| session_not_found(error, session_id))?;
    let actual_directory = canonical_recorded_directory(&info.directory);
    if actual_directory != scope.directory() || info.project_id != expected_project_id {
        return Err(LifecycleError::SessionOwnershipMismatch {
            session_id: session_id.to_string(),
            expected_directory: scope.directory().to_path_buf(),
            actual_directory,
            expected_project_id: expected_project_id.to_string(),
            actual_project_id: info.project_id.clone(),
        });
    }
    Ok(info)
}

async fn resolve_project(scope: &SessionScope) -> Result<(project::Info, String), LifecycleError> {
    project::from_directory(scope.tenant(), scope.directory()).await.map_err(Into::into)
}

/// 创建并持久化一个属于已验证作用域的原生会话。
pub async fn create_session(input: CreateSessionInput) -> Result<SessionSnapshot, LifecycleError> {
    let CreateSessionInput { scope, parent_id, title, permission } = input;
    let (project, _) = resolve_project(&scope).await?;
    let expected_project_id = project.id;
    let tenant = scope.tenant().clone();
    let directory = scope.directory().to_path_buf();
    let directory_string = directory.to_string_lossy().into_owned();
    let scope_for_operation = scope.clone();

    project::instance::provide(directory, None, move || {
        Box::pin(async move {
            ui_store::with_tenant_scope(tenant, async move {
                if let Some(parent_id) = parent_id.as_deref() {
                    load_info_for_scope(&scope_for_operation, &expected_project_id, parent_id)
                        .await?;
                }
                let info = session::create_next(session::CreateInput {
                    parent_id,
                    title,
                    directory: directory_string,
                    permission,
                })
                .await
                .map_err(LifecycleError::from)?;
                Ok(SessionSnapshot { info, history: Vec::new() })
            })
            .await
        })
    })
    .await
    .map_err(LifecycleError::Project)?
}

/// 创建并持久化一个带稳定 slug 标记的用户可见任务会话。
pub async fn create_visible_task_session(
    input: CreateSessionInput,
) -> Result<SessionSnapshot, LifecycleError> {
    let scope = input.scope.clone();
    let created = create_session(input).await?;
    let session_id = created.info.id.clone();
    let tenant = scope.tenant().clone();
    let directory = scope.directory().to_path_buf();

    let info = project::instance::provide(directory, None, move || {
        Box::pin(async move {
            ui_store::with_tenant_scope(tenant, async move {
                session::update(&session_id, |info| {
                    info.slug = session::Info::visible_task_slug(&info.slug);
                })
                .await
                .map_err(LifecycleError::from)
            })
            .await
        })
    })
    .await
    .map_err(LifecycleError::Project)??;

    Ok(SessionSnapshot { info, history: Vec::new() })
}

/// 加载会话，在读取历史前校验租户、目录和项目归属。
pub async fn load_session(
    scope: SessionScope,
    session_id: &str,
) -> Result<SessionSnapshot, LifecycleError> {
    let (project, _) = resolve_project(&scope).await?;
    let expected_project_id = project.id;
    let tenant = scope.tenant().clone();
    let directory = scope.directory().to_path_buf();
    let session_id = session_id.to_string();

    project::instance::provide(directory, None, move || {
        Box::pin(async move {
            ui_store::with_tenant_scope(tenant, async move {
                let info = load_info_for_scope(&scope, &expected_project_id, &session_id).await?;
                let history =
                    message::messages(&session_id, None).await.map_err(LifecycleError::from)?;
                Ok(SessionSnapshot { info, history })
            })
            .await
        })
    })
    .await
    .map_err(LifecycleError::Project)?
}
