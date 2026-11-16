use std::collections::HashMap;
use crate::presentation::*;
#[derive(Debug, Clone)]
pub enum Message {
    WorkSidebarToggled,
    WorkSidebarFilterToggled,
    WorkSidebarFilterClosed,
    WorkSidebarMoreOpened,
    WorkSidebarMoreClosed,
    WorkSidebarProjectNavHovered(bool),
    WorkSidebarFilterScopeSelected(WorkSidebarFilterScope),
    WorkSidebarFilterStatusSelected(WorkSidebarFilterStatus),
    WorkSidebarFilterTimeSelected(WorkSidebarFilterTime),
    WorkSidebarFilterReset,
    WorkSidebarTabSelected(WorkSidebarTab),
    WorkSidebarLoadMorePressed,
    WorkSidebarSpaceTasksLoadMorePressed(String),
    WorkSpacePickerToggled,
    WorkSpacePickerClosed,
}
pub struct State {
    pub work_sidebar_visible_tasks: usize,
    pub work_sidebar_visible_spaces: usize,
    pub work_sidebar_space_task_visible: HashMap<String, usize>,
    pub work_sidebar_project_nav_hovered: bool,
    pub work_sidebar_filter_open: bool,
    pub work_sidebar_more_open: bool,
    pub work_sidebar_collapsed: bool,
    pub work_sidebar_filter_scope: WorkSidebarFilterScope,
    pub work_sidebar_filter_status: WorkSidebarFilterStatus,
    pub work_sidebar_filter_time: WorkSidebarFilterTime,
    pub work_sidebar_tab: WorkSidebarTab,
    pub work_space_picker_open: bool,
}
fn reset_visible_items(app: &mut State) {
    app.work_sidebar_visible_tasks = WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS;
    app.work_sidebar_visible_spaces = WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS;
    app.work_sidebar_space_task_visible.clear();
}

pub fn update(app: &mut State, message: Message) -> Option<bool> {
    match message {
        Message::WorkSidebarToggled => {
            app.work_sidebar_project_nav_hovered = false;
            app.work_sidebar_filter_open = false;
            app.work_sidebar_more_open = false;
            app.work_sidebar_collapsed = !app.work_sidebar_collapsed;
            return Some(app.work_sidebar_collapsed);
        }
        Message::WorkSidebarFilterToggled => {
            app.work_sidebar_filter_open = !app.work_sidebar_filter_open;
            app.work_sidebar_more_open = false;
        }
        Message::WorkSidebarFilterClosed => {
            app.work_sidebar_filter_open = false;
        }
        Message::WorkSidebarMoreOpened => {
            app.work_sidebar_more_open = true;
            app.work_sidebar_filter_open = false;
        }
        Message::WorkSidebarMoreClosed => {
            app.work_sidebar_more_open = false;
        }
        Message::WorkSidebarProjectNavHovered(hovered) => {
            app.work_sidebar_project_nav_hovered = hovered;
        }
        Message::WorkSidebarFilterScopeSelected(scope) => {
            app.work_sidebar_filter_scope = scope;
            reset_visible_items(app);
        }
        Message::WorkSidebarFilterStatusSelected(status) => {
            app.work_sidebar_filter_status = status;
            reset_visible_items(app);
        }
        Message::WorkSidebarFilterTimeSelected(time) => {
            app.work_sidebar_filter_time = time;
            reset_visible_items(app);
        }
        Message::WorkSidebarFilterReset => {
            app.work_sidebar_filter_scope = WorkSidebarFilterScope::All;
            app.work_sidebar_filter_status = WorkSidebarFilterStatus::All;
            app.work_sidebar_filter_time = WorkSidebarFilterTime::All;
            reset_visible_items(app);
        }
        Message::WorkSidebarTabSelected(tab) => {
            app.work_sidebar_tab = tab;
        }
        Message::WorkSidebarLoadMorePressed => match app.work_sidebar_tab {
            WorkSidebarTab::Tasks => {
                app.work_sidebar_visible_tasks =
                    app.work_sidebar_visible_tasks.saturating_add(WORK_SIDEBAR_LOAD_MORE_STEP);
            }
            WorkSidebarTab::Spaces => {
                app.work_sidebar_visible_spaces =
                    app.work_sidebar_visible_spaces.saturating_add(WORK_SIDEBAR_LOAD_MORE_STEP);
            }
        },
        Message::WorkSidebarSpaceTasksLoadMorePressed(project_path) => {
            let current = app
                .work_sidebar_space_task_visible
                .get(&project_path)
                .copied()
                .unwrap_or(WORK_SIDEBAR_SPACE_TASKS_INITIAL_VISIBLE);
            app.work_sidebar_space_task_visible.insert(
                project_path,
                current.saturating_add(WORK_SIDEBAR_SPACE_TASKS_LOAD_MORE_STEP),
            );
        }
        Message::WorkSpacePickerToggled => {
            app.work_space_picker_open = !app.work_space_picker_open;
        }
        Message::WorkSpacePickerClosed => {
            app.work_space_picker_open = false;
        }
    }

    None
}

impl Default for State {
    fn default() -> Self {
        Self {
            work_sidebar_visible_tasks: WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS,
            work_sidebar_visible_spaces: WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS,
            work_sidebar_space_task_visible: HashMap::new(),
            work_sidebar_project_nav_hovered: false,
            work_sidebar_filter_open: false,
            work_sidebar_more_open: false,
            work_sidebar_collapsed: false,
            work_sidebar_filter_scope: WorkSidebarFilterScope::default(),
            work_sidebar_filter_status: WorkSidebarFilterStatus::default(),
            work_sidebar_filter_time: WorkSidebarFilterTime::default(),
            work_sidebar_tab: WorkSidebarTab::default(),
            work_space_picker_open: false,
        }
    }
}

/// Decide whether project-home navigation must release the active session view.
pub fn project_home(screen: crate::ui_types::Screen, has_project: bool, has_session: bool, workspace: &mut super::work_workspace::State) -> bool {
 if screen != crate::ui_types::Screen::Project || !has_project { return false; }
 workspace.work_resource_half_fullscreen = false;
 has_session
}
