//! Host projection and cross-domain effect wiring for provider forms.
use crate::app::{App, Message, message::SettingsMessage};
use desktop_provider_apps::{Scope, message::provider_update::{Effect, State}};
use iced::Task;
use vw_gateway_client::GatewayClient;

fn mapped(effect: Effect) -> Message {
    Message::Settings(SettingsMessage::AppProviderEffect(effect))
}
fn run(app: &mut App, update: impl FnOnce(&mut State, &str, Option<&desktop_provider_apps::state::list::Snapshot>) -> Task<Effect>) -> Task<Message> {
    let list = &mut app.app_providers;
    let mut state = State {
        scope: list.scope.clone(), generation: list.generation, loading: list.loading,
        editor:list.editor.take(),icons:list.icons.take(),env:list.env.take(),endpoints:list.endpoints.take(),
        other_modal_open:list.preview.is_some() || list.snippets.is_some() || list.universal.is_some(),
        form: list.form.take(), desktop: list.desktop.take(), new_draft: list.new_draft.take(),
    };
    let task = update(&mut state, &app.apps_catalog.token, list.snapshot.as_ref());
    list.editor=state.editor; list.icons=state.icons; list.env=state.env; list.endpoints=state.endpoints;
    list.generation = state.generation;
    list.loading = state.loading;
    list.form = state.form;
    list.desktop = state.desktop;
    list.new_draft = state.new_draft;
    task.map(mapped)
}
pub fn edit(app: &mut App, current: Option<(GatewayClient, Scope)>, event: super::edit::EditMessage) -> Task<Message> {
    run(app, |state, token, snapshot| desktop_provider_apps::message::edit::update(state, current, token, snapshot, event))
}
pub fn desktop(app: &mut App, current: Option<(GatewayClient, Scope)>, binding: bool, event: super::desktop::Event) -> Task<Message> {
    run(app, |state, token, _| desktop_provider_apps::message::desktop::update(state, current, token, binding, event))
}
pub fn change_membership(app: &mut App, current: Option<(GatewayClient, Scope)>, id: String, add: bool) -> Task<Message> {
    let editor_open = app.app_providers.editor.is_some();
    run(app, |state, token, snapshot| desktop_provider_apps::message::dsh::change_membership(state, current, token, snapshot, editor_open, id, add))
}
pub fn membership_changed(app: &mut App, current: Option<(GatewayClient, Scope)>, scope: Scope, generation: u64, add: bool, result: Result<vw_api_app_provider_terminal::dsh::DshLiveResult,String>) -> Task<Message> {
    run(app, |state, _, _| desktop_provider_apps::message::dsh::membership_changed(state, current, scope, generation, add, result))
}
pub fn effect(app: &mut App, effect: Effect) -> Task<Message> {
    match effect {
        Effect::List(event) => list(app,event),
        Effect::None => Task::none(),
        Effect::OpenList(id) => super::list::update(app,super::list::ListMessage::Open(id)),
        Effect::Endpoints(event) => super::endpoints::update(app,event),
        Effect::Env(event) => super::env::update(app,event),
        Effect::Icons(event) => super::icons::update(app,event),
        Effect::HermesMemberChanged {scope,generation,add,result} => super::hermes::membership_changed(app,scope,generation,add,result),
        Effect::KimiMemberChanged {scope,generation,add,result} => super::kimi::membership_changed(app,scope,generation,add,result),
        Effect::Desktop(event) => super::desktop::update(app, event),
        Effect::DshMemberChanged {scope,generation,add,result} => super::dsh::membership_changed(app,scope,generation,add,result),
        Effect::Error(text) => app.show_error_toast(text),
        Effect::Info(text) => app.show_info_toast(text),
        Effect::Success(text) => app.show_success_toast(text),
        Effect::Warning(text) => app.show_warning_toast(text),
        Effect::Refresh {scope,generation} => {
            if !request_is_current(app,&scope,generation) {return Task::none()}
            super::list::update(app, super::list::ListMessage::Refresh)
        },
        Effect::Load {client,scope,generation,id,copy,delete,claude,token} => {
            if !request_is_current(app,&scope,generation) || !app.app_providers.loading { return Task::none() }
            super::edit::load(client,scope,generation,id,copy,delete,claude,token)
        },
        Effect::Save {client,scope,generation,form,token} => {
            if !request_is_current(app,&scope,generation) || !app.app_providers.form.as_ref().is_some_and(|f|f.saving) { return Task::none() }
            super::edit::dispatch_save(client,scope,generation,form,token)
        },
        Effect::Field(field) => {
            let Some(form) = app.app_providers.form.as_mut().filter(|f| !f.saving) else { return Task::none() };
            use super::edit::EditMessage as E;
            match field {
                E::Claude(field) => super::claude::change(form,field),
                E::Codex(field) => super::codex::change(form,field),
                E::Gemini(field) => super::gemini::change(form,field),
                E::Grok(field) => super::grok::change(form,field),
                E::Hermes(field) => super::hermes::change(form,field),
                E::Pi(field) => super::pi::change(form,field),
                E::Kimi(field) => super::kimi::change(form,field),
                E::Mcode(field) => super::mcode::change(form,field),
                E::Agy(field) => super::agy::change(form,field),
                E::Dsh(field) => super::dsh::change(form,field),
                E::Omp(field) => super::omp::change(form,field),
                E::OpenClaw(field) => super::openclaw::change(form,field),
                E::OpenCode(field) => super::opencode::change(form,field),
                _ => {}
            }
            Task::none()
        }
    }
}

fn request_is_current(app:&App,scope:&Scope,generation:u64)->bool {
 app.app_providers.scope.as_ref()==Some(scope) && app.app_providers.generation==generation
 && super::list::current(app,scope.app).ok().is_some_and(|(_,now)|now==*scope)
}

pub fn endpoints(app:&mut App,current:Option<(GatewayClient,Scope)>,event:super::endpoints::Event)->Task<Message>{
 run(app,|state,token,_|desktop_provider_apps::message::endpoints::update(state,current,token,event))
}

pub fn env(app:&mut App,current:Option<(GatewayClient,Scope)>,event:super::env::Event)->Task<Message>{
 run(app,|state,token,_|desktop_provider_apps::message::env::update(state,current,token,event))
}

pub fn icons(app:&mut App,current:Result<(GatewayClient,Scope),String>,event:super::icons::Event)->Task<Message>{
 run(app,|state,_,snapshot|desktop_provider_apps::message::icons::update(state,current,snapshot,event))
}

pub fn hermes_change(app:&mut App,current:Option<(GatewayClient,Scope)>,id:String,add:bool)->Task<Message>{
 let editor=app.app_providers.editor.is_some(); run(app,|state,token,snapshot|desktop_provider_apps::message::hermes::change_membership(state,current,token,snapshot,editor,id,add))
}
pub fn hermes_changed(app:&mut App,current:Option<(GatewayClient,Scope)>,scope:Scope,generation:u64,add:bool,result:Result<vw_api_app_provider_extended::hermes::HermesLiveResult,String>)->Task<Message>{
 run(app,|state,_,_|desktop_provider_apps::message::hermes::membership_changed(state,current,scope,generation,add,result))
}

pub fn kimi_change(app:&mut App,current:Option<(GatewayClient,Scope)>,id:String,add:bool)->Task<Message>{
 let editor=app.app_providers.editor.is_some(); run(app,|state,token,snapshot|desktop_provider_apps::message::kimi::change_membership(state,current,token,snapshot,editor,id,add))
}
pub fn kimi_changed(app:&mut App,current:Option<(GatewayClient,Scope)>,scope:Scope,generation:u64,add:bool,result:Result<vw_api_app_provider_terminal::kimi::KimiLiveResult,String>)->Task<Message>{
 run(app,|state,_,_|desktop_provider_apps::message::kimi::membership_changed(state,current,scope,generation,add,result))
}

pub fn list(app:&mut App,event:desktop_provider_apps::message::list::Event)->Task<Message>{
let current=app.app_providers.scope.as_ref().ok_or_else(||"无法连接网关".to_string()).and_then(|s|super::list::current(app,s.app));
run(app,|state,_,snapshot|desktop_provider_apps::message::list::update(state,current,snapshot,event))
}
