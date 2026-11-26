//! Bridge compact companion controls to the ordinary chat and voice workflows.
use super::ViewMessage;
use crate::app::{App, Message};
use desktop_task_board::pet_view::VoiceOrbAction;
use iced::Task;

pub(super) fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    match message {
        ViewMessage::TaskPetDockHover(value) => app.pet_state.controls.hovered = value,
        ViewMessage::TaskPetWriteMessage => {
            app.companion_navigation.task_pet_context_menu_pos = None;
            let controls = &mut app.pet_state.controls;
            if !controls.composer && (controls.draft.is_empty() || controls.project_path.is_none()) {
                controls.project_path = app.navigation_tabs.project_path.clone();
            }
            controls.composer = true;
            app.pet_state.task_pet_reply_request_id = None;
            app.pet_state.task_pet_reply_input.clear();
            return iced::widget::operation::focus(desktop_task_board::task_pet::composer_id());
        }
        ViewMessage::TaskPetComposerChanged(value) if app.pet_state.controls.composer => {
            app.pet_state.controls.draft = value;
        }
        ViewMessage::TaskPetComposerClose => app.pet_state.controls.close_composer(),
        ViewMessage::TaskPetComposerSubmit => return submit(app),
        ViewMessage::TaskPetProjectMenuToggle if app.pet_state.controls.composer => {
            let controls = &mut app.pet_state.controls;
            if controls.project_query.is_some() {
                return close_project_menu(app);
            }
            if controls.project_path.is_none() {
                controls.project_path = app.navigation_tabs.project_path.clone();
            }
            controls.project_query = Some(String::new());
            return iced::widget::operation::focus(desktop_task_board::task_pet::project_search_id());
        }
        ViewMessage::TaskPetProjectMenuClose => return close_project_menu(app),
        ViewMessage::TaskPetProjectQueryChanged(query) => {
            if let Some(current) = &mut app.pet_state.controls.project_query {
                *current = query;
            }
        }
        ViewMessage::TaskPetProjectSelected(path)
            if app.pet_state.controls.composer && app.pet_state.controls.project_query.is_some() =>
        {
            if app.workspace_state.recent_projects.contains(&path)
                || app.pet_state.controls.project_path.as_ref() == Some(&path)
            {
                app.pet_state.controls.project_path = Some(path);
                return close_project_menu(app);
            }
        }
        ViewMessage::TaskPetToggleReminders => {
            let open =
                !app.pet_state.controls.reminders_open && !app.pet_state.task_pet_items.is_empty();
            if !open {
                close_reminders(app);
            } else {
                app.pet_state.task_pet_collapsed = true;
                app.pet_state.controls.reminders_open = true;
            }
        }
        ViewMessage::TaskPetExpandReminders if app.pet_state.controls.reminders_open => {
            app.pet_state.controls.close_composer();
            app.pet_state.task_pet_collapsed = false;
        }
        ViewMessage::TaskPetOpenReminder(id) if !app.pet_state.task_pet_collapsed => {
            if let Some(item) =
                app.pet_state.task_pet_items.iter().find(|item| item.request_id == id)
            {
                let session = item.session_id.clone();
                close_reminders(app);
                let open = super::open_task_pet_session(app, session);
                return app.reopen_main_window().chain(open);
            }
        }
        ViewMessage::TaskPetStartVoice => return start_voice(app),
        ViewMessage::TaskPetVoiceMic => return voice_action(app, None),
        ViewMessage::TaskPetVoiceOrb(action) => return voice_action(app, Some(action)),
        _ => {}
    }
    Task::none()
}

fn close_project_menu(app: &mut App) -> Task<Message> {
    if app.pet_state.controls.project_query.take().is_some() && app.pet_state.controls.composer {
        iced::widget::operation::focus(desktop_task_board::task_pet::composer_id())
    } else {
        Task::none()
    }
}

pub(super) fn blur(app: &mut App) {
    app.pet_state.controls.close_composer();
    app.pet_state.controls.hovered = false;
    app.companion_navigation.task_pet_robot_hovered = false;
    app.companion_navigation.task_pet_context_menu_pos = None;
}

pub(super) fn close_reminders(app: &mut App) {
    app.pet_state.controls.reminders_open = false;
    app.pet_state.task_pet_hovered_request_id = None;
    app.pet_state.task_pet_reply_request_id = None;
    app.pet_state.task_pet_reply_input.clear();
}

pub(super) fn hide(app: &mut App) -> Task<Message> {
    blur(app);
    close_reminders(app);
    app.pet_state.task_pet_collapsed = true;
    let stop = voice_action(app, Some(VoiceOrbAction::End));
    app.pet_state.controls = Default::default();
    stop
}

fn submit(app: &mut App) -> Task<Message> {
    if !app.pet_state.controls.composer {
        return Task::none();
    }
    let input = app.pet_state.controls.draft.trim().to_owned();
    if input.is_empty() {
        return Task::none();
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let current = app.navigation_tabs.project_path.clone();
        let Some(project) = app.pet_state.controls.project_path.clone()
            .or_else(|| current.clone())
            .or_else(desktop_workspace::basic::pick_folder)
        else {
            return Task::none();
        };
        let stop = voice_action(app, Some(VoiceOrbAction::End));
        blur(app);
        close_reminders(app);
        app.pet_state.controls.draft.clear();
        app.pet_state.controls.project_path = None;

        let directory = if current.as_ref() == Some(&project) {
            app.session_state
                .active_session_id
                .as_deref()
                .and_then(|id| app.known_session_directory(id))
                .or_else(|| {
                    app.current_session_runtime_ref()
                        .and_then(|runtime| runtime.draft.as_ref())
                        .filter(|draft| draft.project_path == project)
                        .map(|draft| draft.directory.clone())
                })
                .unwrap_or_else(|| project.clone())
        } else {
            project.clone()
        };
        if let Some(id) = app.session_state.active_session_id.clone() {
            let files = std::mem::take(&mut app.session_state.files);
            app.get_session_runtime_mut(&id).parked_input_files = Some(files);
        }
        let switch = if current.as_ref() == Some(&project) {
            app.navigation_tabs.show_project();
            Task::none()
        } else {
            app.open_project_and_index(project.clone())
        };
        app.navigation_panels.focus_chat();
        let draft = crate::app::message::project::open_session_draft(app, project, directory);
        // The compact composer sends just its own text, even if the old
        // composer parked attachments or a draft before this window opened.
        let editor = iced::widget::text_editor::Content::with_text(&input);
        let runtime = app.current_session_runtime_mut();
        runtime.input_editor = editor.clone();
        runtime.task_mode_enabled = false;
        runtime.workflow_mode_enabled = false;
        runtime.pending_media_model = None;
        runtime.tool_selector.clear_manual_context_selection();
        app.session_state.input_editor = editor;
        app.session_state.files.clear();
        let show = app.reopen_main_window();
        let send =
            crate::app::message::chat::update(app, crate::app::message::ChatMessage::SendPressed);
        Task::batch([stop, switch, draft, show, send])
    }
    #[cfg(target_arch = "wasm32")]
    Task::none()
}

#[cfg(not(target_arch = "wasm32"))]
fn start_voice(app: &mut App) -> Task<Message> {
    use crate::app::message::voice;
    if app.voice.chat_active() {
        return app.reopen_main_window();
    }
    if app.navigation_tabs.project_path.is_none() {
        let show = app.reopen_main_window();
        let feedback = app.show_localized_toast(
            crate::app::state::ToastKind::Info,
            "请先选择聊天会话。",
            vec![],
        );
        return Task::batch([show, feedback]);
    }
    voice::invalidate(app);
    let task = voice::start_chat(app, true);
    app.pet_state.controls.voice_owned = app.voice.chat_active();
    if app.pet_state.controls.voice_owned {
        task
    } else {
        Task::batch([task, app.reopen_main_window()])
    }
}

#[cfg(target_arch = "wasm32")]
fn start_voice(_: &mut App) -> Task<Message> {
    Task::none()
}

fn voice_action(app: &mut App, action: Option<VoiceOrbAction>) -> Task<Message> {
    #[cfg(not(target_arch = "wasm32"))]
    if app.pet_state.controls.voice_owned {
        use crate::app::message::voice::{self, Event, local_chat};
        let event = match action {
            None => Event::ChatMute,
            Some(VoiceOrbAction::Begin) => Event::Local(local_chat::Event::Begin),
            Some(VoiceOrbAction::Finish) => Event::Local(local_chat::Event::Finish),
            Some(VoiceOrbAction::Interrupt) => Event::ChatInterrupt,
            Some(VoiceOrbAction::End) => {
                app.pet_state.controls.voice_owned = false;
                Event::ChatEnd
            }
        };
        return voice::update(app, event);
    }
    let _ = (app, action);
    Task::none()
}
