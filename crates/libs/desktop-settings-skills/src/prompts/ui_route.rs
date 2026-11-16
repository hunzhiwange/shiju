use super::Event;
pub enum UiRoute {
    Update(super::update::Event),
    Agents,
    Projects,
}
pub fn ui_route(event: Event) -> UiRoute {
    use super::Event as Ui;
    use super::update::{self as host, Event};
    UiRoute::Update(match event {
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
            super::sync::Event::Open => host::sync::Event::Open,
            super::sync::Event::Close => host::sync::Event::Close,
            super::sync::Event::Start => host::sync::Event::Start,
            super::sync::Event::Refresh => host::sync::Event::Refresh,
            super::sync::Event::Cancel => host::sync::Event::Cancel,
            super::sync::Event::Retry => host::sync::Event::Retry,
        }),
        Ui::Template(event) => Event::Template(match event {
            super::templates::Event::Open => host::templates::Event::Open,
            super::templates::Event::Close => host::templates::Event::Close,
            super::templates::Event::Load => host::templates::Event::Load,
            super::templates::Event::Edit(v0) => host::templates::Event::Edit(v0),
            super::templates::Event::Cancel => host::templates::Event::Cancel,
            super::templates::Event::Slug(v0) => host::templates::Event::Slug(v0),
            super::templates::Event::Body(v0) => host::templates::Event::Body(v0),
            super::templates::Event::Save => host::templates::Event::Save,
            super::templates::Event::Delete(v0) => host::templates::Event::Delete(v0),
            super::templates::Event::Undo => host::templates::Event::Undo,
            super::templates::Event::Copy(v0) => host::templates::Event::Copy(v0),
        }),
        Ui::PiFile(event) => Event::PiFile(match event {
            super::pi_files::FileEvent::Open => host::pi_files::Event::Open,
            super::pi_files::FileEvent::Close => host::pi_files::Event::Close,
            super::pi_files::FileEvent::Select(v0) => host::pi_files::Event::Select(v0),
            super::pi_files::FileEvent::Edit(v0) => host::pi_files::Event::Edit(v0),
            super::pi_files::FileEvent::Read => host::pi_files::Event::Read,
            super::pi_files::FileEvent::Save => host::pi_files::Event::Save,
            super::pi_files::FileEvent::Delete => host::pi_files::Event::Delete,
            super::pi_files::FileEvent::AcceptRevision => host::pi_files::Event::AcceptRevision,
        }),
        Ui::SharedImport(event) => Event::SharedImport(match event {
            super::shared_import::ImportEvent::Cancel => host::shared_import::Event::Cancel,
            super::shared_import::ImportEvent::Enable(v0) => host::shared_import::Event::Enable(v0),
            super::shared_import::ImportEvent::Confirm => host::shared_import::Event::Confirm,
        }),
        Ui::Transfer(event) => Event::Transfer(match event {
            super::transfer::TransferEvent::Open(v0) => host::transfer::Event::Open(v0),
            super::transfer::TransferEvent::Close => host::transfer::Event::Close,
            super::transfer::TransferEvent::Select(v0, v1) => host::transfer::Event::Select(v0, v1),
            super::transfer::TransferEvent::Copy => host::transfer::Event::Copy,
            super::transfer::TransferEvent::Body(v0) => host::transfer::Event::Body(v0),
            super::transfer::TransferEvent::Path => host::transfer::Event::Path,
            super::transfer::TransferEvent::Reveal => host::transfer::Event::Reveal,
            super::transfer::TransferEvent::ImportHost => host::transfer::Event::ImportHost,
            super::transfer::TransferEvent::ImportClient => host::transfer::Event::ImportClient,
        }),
        Ui::Agents => return UiRoute::Agents,
        Ui::Projects => return UiRoute::Projects,
    })
}
