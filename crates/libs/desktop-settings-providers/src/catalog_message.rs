use vw_api_transport::id::ModelId;
use vw_api_model_catalog::ModelDiscoveryMethod;
#[derive(Debug, Clone)]
pub enum Event {
    Open(String),
    Close,
    Query(String),
    Search,
    ChooseEndpoint { generation: u64, preset: usize },
    Toggle { generation: u64, preset: usize, model: usize, checked: bool },
    Endpoint(String),
    ManualId(String),
    Alias(String),
    MakeDefault(bool),
    EditModel { generation: u64, model_id: ModelId },
    CancelEdit,
    LoadCatalog,
    Save,
    Fill(FillEvent),
    Discovery(DiscoveryEvent),
}
#[derive(Debug, Clone)]
pub enum FillEvent { Preview, Commit, Silent(bool) }
#[derive(Debug, Clone)]
pub enum DiscoveryEvent { Start(ModelDiscoveryMethod), Cancel, Toggle { run: u64, index: usize, checked: bool } }
pub fn message(event: Event) -> crate::view_message::Message { crate::view_message::Message::Catalog(event) }
