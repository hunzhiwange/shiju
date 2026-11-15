//! 待办会话隔离与面板状态更新；返回是否需要动画 tick。
pub struct TodoPanel<'a> {
 pub active_session_id: Option<&'a str>,
 pub chat_todo_session_id: &'a mut Option<String>,
 pub chat_todo_items: &'a mut Vec<todo_types::Todo>,
 pub chat_todo_expanded: &'a mut bool,
 pub chat_todo_anim: &'a mut f32,
}
pub fn maybe_auto_collapse_todo_panel(app: &mut TodoPanel<'_>) -> bool {
    if !(*app.chat_todo_expanded) && (*app.chat_todo_anim) <= 0.001 {
        return false;
    }

    let Some(session) = app.active_session_id else {
        (*app.chat_todo_session_id) = None;
        (*app.chat_todo_items).clear();
        return false;
    };

    if (*app.chat_todo_session_id).as_deref() != Some(session) {
        return false;
    }

    let todos = &(*app.chat_todo_items);
    if todos.is_empty() {
        return false;
    }
    if todos.iter().any(|t| t.status != "completed") {
        return false;
    }

    (*app.chat_todo_expanded) = false;
    if (*app.chat_todo_anim).abs() < 0.001 {
        (*app.chat_todo_anim) = 0.0;
        false
    } else {
        true
    }
}

pub fn apply_loaded_session_todos(
    app: &mut TodoPanel<'_>,
    session_id: String,
    res: Result<Vec<todo_types::Todo>, String>,
) -> bool {
    if app.active_session_id != Some(session_id.as_str()) {
        return false;
    }

    let session_changed = (*app.chat_todo_session_id).as_deref() != Some(session_id.as_str());

    match res {
        Ok(todos) => {
            (*app.chat_todo_session_id) = Some(session_id);
            (*app.chat_todo_items) = todos;
            if session_changed
                && !(*app.chat_todo_items).is_empty()
                && (*app.chat_todo_items).iter().any(|todo| todo.status != "completed")
            {
                (*app.chat_todo_expanded) = true;
                (*app.chat_todo_anim) = 1.0;
            }
        }
        Err(err) => {
            tracing::warn!(target: "vw_desktop", error = %err, session_id = %session_id, "failed to load todo list via gateway");
            (*app.chat_todo_session_id) = Some(session_id);
            (*app.chat_todo_items).clear();
        }
    }

    maybe_auto_collapse_todo_panel(app)
}


#[cfg(test)]
mod tests;
