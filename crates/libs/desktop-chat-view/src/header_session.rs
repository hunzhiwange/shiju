//! 会话标题栏的只读派生状态与操作。
use super::header::{HeaderInput, HeaderAction};
use desktop_i18n::I18n;
use iced::Color;
use desktop_ui_controls::animated_text::mix_color;

fn known_session_info<'a>(
    app: &'a HeaderInput<'_>,
    session_id: &str,
) -> Option<&'a session_info::Info> {
    app.sessions.iter().find(|session| session.id == session_id).or_else(|| {
        app.project_sessions
            .values()
            .flat_map(|sessions| sessions.iter())
            .find(|session| session.id == session_id)
    })
}

pub fn active_session_directory(app: &HeaderInput<'_>) -> Option<String> {
    let session_id = app.active_session_id.as_ref()?;
    known_session_info(app, session_id).map(|session| session.directory.clone())
}

pub fn return_to_parent_session_pressed(app: &HeaderInput<'_>) -> Option<HeaderAction> {
    let session_id = app.active_session_id.as_deref()?;
    let session = known_session_info(app, session_id)?;
    let parent_id = session.parent_id.as_deref()?.trim();
    if parent_id.is_empty() {
        return None;
    }
    let path = app
        .project_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .or_else(|| {
            known_session_info(app, parent_id)
                .map(|parent| parent.directory.clone())
                .filter(|directory| !directory.trim().is_empty())
        })
        .or_else(|| {
            let directory = session.directory.trim();
            (!directory.is_empty()).then(|| directory.to_string())
        })?;
    Some(HeaderAction::OpenParent(path, parent_id.to_string()))
}

pub fn session_workspace_badge_state(app: &HeaderInput<'_>) -> Option<(String, bool)> {
    let directory = active_session_directory(app)?;
    let project_path = app.project_path.as_deref().unwrap_or_default();
    let is_primary = !project_path.is_empty() && directory == project_path;
    let label = if is_primary {
        I18n::new(app.desktop_locale).__("主工作区").to_string()
    } else {
        std::path::Path::new(&directory)
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                I18n::new(app.desktop_locale).__("工作区").to_string()
            })
    };

    Some((label, is_primary))
}

pub fn get_session_status(app: &HeaderInput<'_>) -> (bool, usize, bool, f32, Color) {
    let now_s = app.now_ms as f64 / 1000.0;

    if let Some(session_id) = &app.active_session_id {
        let (is_running, queued, has_unseen_success) = app.runtime;

        // 使用 FNV-1a 哈希算法为会话 ID 生成相位偏移
        let session_phase = session_id
            .bytes()
            .fold(0u32, |acc, b| acc.wrapping_mul(16777619).wrapping_add(b as u32));
        let phase =
            ((now_s / 1.8) + (session_phase as f64 / u32::MAX as f64)) * std::f64::consts::TAU;
        let breathe = ((phase.sin() + 1.0) * 0.5) as f32;
        let running_color =
            mix_color(Color::from_rgb8(245, 158, 11), Color::from_rgb8(239, 68, 68), breathe);

        (is_running, queued, has_unseen_success, breathe, running_color)
    } else {
        (false, 0, false, 0.0, Color::TRANSPARENT)
    }
}

pub fn session_compact_message(app: &HeaderInput<'_>) -> Option<HeaderAction> {
    let session_id = app.active_session_id.as_deref()?;
    if app.runtime.0 {
        return None;
    }
    let session = known_session_info(app, session_id)?;
    if session.time.compacting.is_some() {
        return None;
    }
    Some(HeaderAction::Compact {
        session_id: session_id.to_string(),
        project_path: session.directory.clone(),
    })
}
