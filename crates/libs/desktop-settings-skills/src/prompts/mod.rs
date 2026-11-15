pub mod activation;
pub mod editor;
pub mod list;
pub mod pi_files;
pub mod shared_import;
pub mod sync;
pub mod templates;
pub mod transfer;
pub use list::State;
#[derive(Clone, Debug)]
pub enum Event {
    Open(vw_api_app_identity::AppId),
    Edit(Option<String>),
    CloseEditor,
    Name(String),
    Description(String),
    Body(iced::widget::text_editor::Action),
    Save,
    Mutate(activation::Action),
    Undo,
    Sync(sync::Event),
    Template(templates::Event),
    PiFile(pi_files::FileEvent),
    SharedImport(shared_import::ImportEvent),
    Transfer(transfer::TransferEvent),
    Agents,
    Projects,
}

pub mod update;
