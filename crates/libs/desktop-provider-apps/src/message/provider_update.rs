use crate::{Scope, state::{edit::EditForm, desktop::Form}};
use vw_gateway_client::GatewayClient;
#[derive(Default)]
pub struct State {
 pub scope: Option<Scope>, pub generation: u64, pub loading: bool,
 pub form: Option<EditForm>, pub desktop: Option<Form>,
 pub editor: Option<crate::state::list::Editor>,
 pub icons: Option<crate::state::icons::Form>, pub env: Option<crate::state::env::Form>, pub endpoints: Option<crate::state::endpoints::Form>,
 pub other_modal_open: bool,
 pub new_draft: Option<vw_api_app_provider_base::create::AppProviderCreate>,
}
#[derive(Clone)]
pub enum Effect {
 DshMemberChanged { scope: Scope, generation: u64, add: bool, result: Result<vw_api_app_provider_terminal::dsh::DshLiveResult,String> },
 Desktop(super::desktop::Event),
 Endpoints(super::endpoints::Event), Env(super::env::Event), Icons(super::icons::Event),
 List(super::list::Event),
 None, OpenList(vw_api_app_identity::AppId),
 HermesMemberChanged {scope:Scope,generation:u64,add:bool,result:Result<vw_api_app_provider_extended::hermes::HermesLiveResult,String>},
 KimiMemberChanged {scope:Scope,generation:u64,add:bool,result:Result<vw_api_app_provider_terminal::kimi::KimiLiveResult,String>},
 Error(String), Info(String), Success(String), Warning(String), Refresh { scope: Scope, generation: u64 },
 Load { client: GatewayClient, scope: Scope, generation: u64, id: String, copy: bool, delete: bool, claude: bool, token: String },
 Save { client: GatewayClient, scope: Scope, generation: u64, form: EditForm, token: String },
 Field(super::edit::EditMessage),
}
pub fn error(value: impl Into<String>) -> iced::Task<Effect> { iced::Task::done(Effect::Error(value.into())) }
pub fn info(value: impl Into<String>) -> iced::Task<Effect> { iced::Task::done(Effect::Info(value.into())) }
pub fn success(value: impl Into<String>) -> iced::Task<Effect> { iced::Task::done(Effect::Success(value.into())) }
pub fn warning(value: impl Into<String>) -> iced::Task<Effect> { iced::Task::done(Effect::Warning(value.into())) }
pub fn refresh(state: &State) -> iced::Task<Effect> {
 let Some(scope)=state.scope.clone() else {return iced::Task::none()};
 iced::Task::done(Effect::Refresh {scope,generation:state.generation})
}

impl std::fmt::Debug for Effect {
 fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result { f.write_str("ProviderFormEffect([redacted])") }
}
