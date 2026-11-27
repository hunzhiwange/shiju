use super::work_view_prefs::{filter, load_filter_draft};
use crate::app::{App, Message};
use desktop_i18n::workflow::Feedback;
use desktop_task_board::message::AdvancedWorkFilterAction as Action;
use desktop_task_board::state::State;
use desktop_task_board::work_views::{
    WorkAdvancedFilterDraft, WorkFilterCondition, WorkFilterField, WorkFilterGroup, WorkFilterJoin,
    WorkFilterNode, WorkToolbarMenu, WorkViewFilter,
};
use iced::Task;

#[cfg(test)]
#[path = "advanced_filter_tests.rs"]
mod tests;

pub(super) fn discard(board: &mut State) {
    if board.work_advanced_filter.take().is_some() {
        let applied = board.work_filter.clone();
        load_filter_draft(board, &applied);
    }
}

pub(super) fn update(app: &mut App, action: Action) -> Task<Message> {
    match change(app, action) {
        Ok(task) => task,
        Err(error) => app.show_message_toast(crate::app::state::ToastKind::Error, error.message()),
    }
}

fn change(app: &mut App, action: Action) -> Result<Task<Message>, Feedback> {
    if matches!(action, Action::Open) {
        let mut simple = if app.task_board.work_toolbar_menu == Some(WorkToolbarMenu::Filter) {
            filter(app)?
        } else {
            app.task_board.work_filter.clone()
        };
        let group = simple.advanced.take().unwrap_or_else(|| WorkFilterGroup {
            children: vec![new_condition()],
            ..Default::default()
        });
        let board = &mut app.task_board;
        board.work_advanced_filter =
            Some(WorkAdvancedFilterDraft { simple, group, editing: None, menu: None });
        board.work_toolbar_menu = Some(WorkToolbarMenu::AdvancedFilter);
        board.work_editor_open = false;
        board.work_filter_field = None;
        return Ok(Task::none());
    }
    if app.task_board.work_toolbar_menu != Some(WorkToolbarMenu::AdvancedFilter) {
        return Ok(Task::none());
    }
    let copying_active = matches!(&action, Action::DuplicateCondition(path)
        if app.task_board.work_advanced_filter.as_ref().and_then(|draft| draft.editing.as_ref()) == Some(path));
    if copying_active
        || matches!(action, Action::ConfirmValue | Action::Apply | Action::SwitchToSimple)
    {
        confirm_value(app)?;
    }
    let adding_group = matches!(&action, Action::AddGroup(_));
    let board = &mut app.task_board;
    let Some(draft) = board.work_advanced_filter.as_mut() else {
        return Ok(Task::none());
    };
    draft.menu = None;
    match action {
        Action::Apply | Action::SwitchToSimple => {
            let mut draft = board.work_advanced_filter.take().expect("advanced filter open");
            draft.group.trim_empty();
            draft.simple.advanced = (!draft.group.children.is_empty()).then_some(draft.group);
            load_filter_draft(board, &draft.simple);
            board.work_filter = draft.simple;
            board.task_board_selected_tasks.clear();
            board.work_toolbar_menu = None;
            if matches!(action, Action::SwitchToSimple) {
                board.work_toolbar_menu = Some(WorkToolbarMenu::Filter);
                board.work_filter_query.clear();
                board.work_filter_value_query.clear();
                return Ok(iced::widget::operation::focus(
                    crate::app::views::task_board::WORK_FILTER_SEARCH_INPUT_ID,
                ));
            }
        }
        Action::Clear => {
            draft.group = WorkFilterGroup::default();
            draft.editing = None;
        }
        Action::AddCondition(path) | Action::AddGroup(path) => {
            let node = if adding_group {
                if !path.is_empty() {
                    return Ok(Task::none());
                }
                WorkFilterNode::Group(WorkFilterGroup {
                    join: WorkFilterJoin::Any,
                    children: vec![new_condition()],
                })
            } else {
                new_condition()
            };
            if let Some(group) = group_mut(&mut draft.group, &path) {
                group.children.push(node);
            }
        }
        Action::SetJoin(path, join) => {
            if let Some(group) = group_mut(&mut draft.group, &path) {
                group.join = join;
            }
        }
        Action::SetField(path, field) => {
            if field != WorkFilterField::Advanced
                && let Some(condition) = condition_mut(&mut draft.group, &path)
            {
                if condition.field != field {
                    condition.field = field;
                    condition.rule = WorkViewFilter::default();
                }
                let rule = condition.rule.clone();
                draft.editing = Some(path);
                load_filter_draft(board, &rule);
                board.work_filter_value_query.clear();
            }
        }
        Action::SetExcluded(path, excluded) => {
            if let Some(condition) = condition_mut(&mut draft.group, &path) {
                condition.excluded = excluded;
            }
        }
        Action::ShowMenu(path) => draft.menu = path,
        Action::ConvertToGroup(path) => convert_to_group(draft, &path),
        Action::DuplicateCondition(path) => duplicate_condition(draft, &path),
        Action::Remove(path) => remove_node(draft, &path),
        Action::EditValue(path) => {
            if draft.editing.as_ref() == Some(&path) {
                draft.editing = None;
            } else if let Some(condition) = condition_mut(&mut draft.group, &path) {
                let rule = condition.rule.clone();
                draft.editing = Some(path);
                load_filter_draft(board, &rule);
                board.work_filter_value_query.clear();
            }
        }
        Action::CancelValue => draft.editing = None,
        Action::ConfirmValue | Action::Open => {}
    }
    Ok(Task::none())
}

fn convert_to_group(draft: &mut WorkAdvancedFilterDraft, path: &[usize]) {
    let [index] = path else { return };
    let Some(node @ WorkFilterNode::Condition(_)) = draft.group.children.get_mut(*index) else {
        return;
    };
    *node = WorkFilterNode::Group(WorkFilterGroup {
        join: WorkFilterJoin::Any,
        children: vec![node.clone()],
    });
    if let Some(editing) = draft.editing.as_mut().filter(|editing| editing.as_slice() == path) {
        editing.push(0);
    }
}

fn duplicate_condition(draft: &mut WorkAdvancedFilterDraft, path: &[usize]) {
    let Some((&index, parent)) = path.split_last() else { return };
    let Some(group) = group_mut(&mut draft.group, parent) else { return };
    let Some(node @ WorkFilterNode::Condition(_)) = group.children.get(index) else { return };
    group.children.insert(index + 1, node.clone());
    if let Some(edited) = draft
        .editing
        .as_mut()
        .filter(|editing| editing.starts_with(parent))
        .and_then(|editing| editing.get_mut(parent.len()))
        && *edited > index
    {
        *edited += 1;
    }
}

fn remove_node(draft: &mut WorkAdvancedFilterDraft, path: &[usize]) {
    let Some((&index, parent)) = path.split_last() else { return };
    let Some(group) = group_mut(&mut draft.group, parent) else { return };
    if index >= group.children.len() {
        return;
    }
    group.children.remove(index);
    if let Some(edited) = draft
        .editing
        .as_mut()
        .filter(|editing| editing.starts_with(parent))
        .and_then(|editing| editing.get_mut(parent.len()))
    {
        if *edited == index {
            draft.editing = None;
        } else if *edited > index {
            *edited -= 1;
        }
    }
}

fn confirm_value(app: &mut App) -> Result<(), Feedback> {
    if app
        .task_board
        .work_advanced_filter
        .as_ref()
        .and_then(|draft| draft.editing.as_ref())
        .is_none()
    {
        return Ok(());
    }
    let mut rule = filter(app)?;
    rule.advanced = None;
    if let Some(draft) = app.task_board.work_advanced_filter.as_mut()
        && let Some(path) = draft.editing.take()
        && let Some(condition) = condition_mut(&mut draft.group, &path)
    {
        condition.rule = rule;
    }
    Ok(())
}

fn new_condition() -> WorkFilterNode {
    WorkFilterNode::Condition(WorkFilterCondition {
        field: WorkFilterField::Status,
        excluded: false,
        rule: WorkViewFilter::default(),
    })
}

fn group_mut<'a>(
    mut group: &'a mut WorkFilterGroup,
    path: &[usize],
) -> Option<&'a mut WorkFilterGroup> {
    for &index in path {
        let WorkFilterNode::Group(nested) = group.children.get_mut(index)? else {
            return None;
        };
        group = nested;
    }
    Some(group)
}

fn condition_mut<'a>(
    group: &'a mut WorkFilterGroup,
    path: &[usize],
) -> Option<&'a mut WorkFilterCondition> {
    let (&index, parent) = path.split_last()?;
    match group_mut(group, parent)?.children.get_mut(index)? {
        WorkFilterNode::Condition(condition) => Some(condition),
        WorkFilterNode::Group(_) => None,
    }
}
