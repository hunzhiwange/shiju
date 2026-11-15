//! Native prompt library and scoped editor.
pub(crate) mod list;
pub(crate) use list::State;
pub(crate) mod activation;
pub(crate) mod editor;

pub(crate) mod pi_files;

pub(crate) mod sync;

pub(crate) mod templates;

pub(crate) mod transfer;

pub(crate) mod shared_import;

pub(crate) fn ui_message(event: desktop_settings_skills::prompts::Event) -> crate::app::Message {
    use crate::app::message::settings::prompts::{self as host, Event};
    use desktop_settings_skills::prompts::Event as Ui;
    host::message(match event {
        Ui::Open(v) => Event::Open(v),
        Ui::Edit(v) => Event::Edit(v),
        Ui::CloseEditor => Event::CloseEditor,
        Ui::Name(v) => Event::Name(v),
        Ui::Description(v) => Event::Description(v),
        Ui::Body(v) => Event::Body(v),
        Ui::Save => Event::Save,
        Ui::Mutate(v) => Event::Mutate(v),
        Ui::Undo => Event::Undo,
        Ui::Sync(event) => Event::Sync(match event {
            desktop_settings_skills::prompts::sync::Event::Open => host::sync::Event::Open,
            desktop_settings_skills::prompts::sync::Event::Close => host::sync::Event::Close,
            desktop_settings_skills::prompts::sync::Event::Start => host::sync::Event::Start,
            desktop_settings_skills::prompts::sync::Event::Refresh => host::sync::Event::Refresh,
            desktop_settings_skills::prompts::sync::Event::Cancel => host::sync::Event::Cancel,
            desktop_settings_skills::prompts::sync::Event::Retry => host::sync::Event::Retry,
        }),
        Ui::Template(event) => Event::Template(match event {
            desktop_settings_skills::prompts::templates::Event::Open => {
                host::templates::Event::Open
            }
            desktop_settings_skills::prompts::templates::Event::Close => {
                host::templates::Event::Close
            }
            desktop_settings_skills::prompts::templates::Event::Load => {
                host::templates::Event::Load
            }
            desktop_settings_skills::prompts::templates::Event::Edit(v0) => {
                host::templates::Event::Edit(v0)
            }
            desktop_settings_skills::prompts::templates::Event::Cancel => {
                host::templates::Event::Cancel
            }
            desktop_settings_skills::prompts::templates::Event::Slug(v0) => {
                host::templates::Event::Slug(v0)
            }
            desktop_settings_skills::prompts::templates::Event::Body(v0) => {
                host::templates::Event::Body(v0)
            }
            desktop_settings_skills::prompts::templates::Event::Save => {
                host::templates::Event::Save
            }
            desktop_settings_skills::prompts::templates::Event::Delete(v0) => {
                host::templates::Event::Delete(v0)
            }
            desktop_settings_skills::prompts::templates::Event::Undo => {
                host::templates::Event::Undo
            }
            desktop_settings_skills::prompts::templates::Event::Copy(v0) => {
                host::templates::Event::Copy(v0)
            }
        }),
        Ui::PiFile(event) => Event::PiFile(match event {
            desktop_settings_skills::prompts::pi_files::FileEvent::Open => {
                host::pi_files::Event::Open
            }
            desktop_settings_skills::prompts::pi_files::FileEvent::Close => {
                host::pi_files::Event::Close
            }
            desktop_settings_skills::prompts::pi_files::FileEvent::Select(v0) => {
                host::pi_files::Event::Select(v0)
            }
            desktop_settings_skills::prompts::pi_files::FileEvent::Edit(v0) => {
                host::pi_files::Event::Edit(v0)
            }
            desktop_settings_skills::prompts::pi_files::FileEvent::Read => {
                host::pi_files::Event::Read
            }
            desktop_settings_skills::prompts::pi_files::FileEvent::Save => {
                host::pi_files::Event::Save
            }
            desktop_settings_skills::prompts::pi_files::FileEvent::Delete => {
                host::pi_files::Event::Delete
            }
            desktop_settings_skills::prompts::pi_files::FileEvent::AcceptRevision => {
                host::pi_files::Event::AcceptRevision
            }
        }),
        Ui::SharedImport(event) => Event::SharedImport(match event {
            desktop_settings_skills::prompts::shared_import::ImportEvent::Cancel => {
                host::shared_import::Event::Cancel
            }
            desktop_settings_skills::prompts::shared_import::ImportEvent::Enable(v0) => {
                host::shared_import::Event::Enable(v0)
            }
            desktop_settings_skills::prompts::shared_import::ImportEvent::Confirm => {
                host::shared_import::Event::Confirm
            }
        }),
        Ui::Transfer(event) => Event::Transfer(match event {
            desktop_settings_skills::prompts::transfer::TransferEvent::Open(v0) => {
                host::transfer::Event::Open(v0)
            }
            desktop_settings_skills::prompts::transfer::TransferEvent::Close => {
                host::transfer::Event::Close
            }
            desktop_settings_skills::prompts::transfer::TransferEvent::Select(v0, v1) => {
                host::transfer::Event::Select(v0, v1)
            }
            desktop_settings_skills::prompts::transfer::TransferEvent::Copy => {
                host::transfer::Event::Copy
            }
            desktop_settings_skills::prompts::transfer::TransferEvent::Body(v0) => {
                host::transfer::Event::Body(v0)
            }
            desktop_settings_skills::prompts::transfer::TransferEvent::Path => {
                host::transfer::Event::Path
            }
            desktop_settings_skills::prompts::transfer::TransferEvent::Reveal => {
                host::transfer::Event::Reveal
            }
            desktop_settings_skills::prompts::transfer::TransferEvent::ImportHost => {
                host::transfer::Event::ImportHost
            }
            desktop_settings_skills::prompts::transfer::TransferEvent::ImportClient => {
                host::transfer::Event::ImportClient
            }
        }),
        Ui::Agents => {
            return crate::app::Message::Settings(
                crate::app::message::SettingsMessage::SystemTabSelected(
                    crate::app::components::system_settings::SystemTab::Agents,
                ),
            );
        }
        Ui::Projects => {
            return crate::app::Message::Settings(
                crate::app::message::SettingsMessage::SystemTabSelected(
                    crate::app::components::system_settings::SystemTab::Projects,
                ),
            );
        }
    })
}
