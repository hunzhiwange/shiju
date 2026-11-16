//! 渲染从最近项目列表移除项目的确认对话框。


#[derive(Clone, Debug)]
pub enum Event { RecentRemoveConfirmed(String), RecentRemoveCanceled }

use desktop_ui_controls::toast::confirm_dialog;
use iced::Element;

pub fn with_recent_project_remove_confirm<'a, Message: Clone + 'static>(
    path: Option<&String>, recent_projects: &[String], recent_projects_edits: &[String], recent_projects_meta: &[crate::project_message::edit_meta::RecentProjectMeta], locale: config_ui::DesktopLocale,
    emit: impl Fn(Event) -> Message + Copy + 'a,
    root_content: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(path) = path else {
        return root_content;
    };

    let display_name = recent_projects
        .iter()
        .position(|p| p == path)
        .and_then(|idx| recent_projects_edits.get(idx))
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .or_else(|| {
            recent_projects_meta
                .iter()
                .find(|meta| &meta.path == path)
                .map(|meta| meta.name.trim().to_string())
                .filter(|name| !name.is_empty())
        })
        .unwrap_or_else(|| {
            std::path::Path::new(path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(path)
                .to_string()
        });

    let confirm = emit(Event::RecentRemoveConfirmed(path.clone()));
    let cancel = emit(Event::RecentRemoveCanceled);
    let dialog = confirm_dialog(
        desktop_i18n::I18n::new(locale).__("确认移除项目"),
        desktop_i18n::I18n::new(locale)
            .format("确定从列表中移除「%s」吗？这不会删除磁盘上的文件。", &[display_name.into()])
            .expect("valid removal template"),
        desktop_i18n::I18n::new(locale).__("确认移除"),
        desktop_i18n::I18n::new(locale).__("取消"),
        confirm,
        cancel,
    );

    iced::widget::stack![root_content, dialog].into()
}
