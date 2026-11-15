pub use desktop_provider_apps::message::paths::{Event,available};
use crate::app::{App,Message};
use desktop_provider_apps::Scope;
use iced::Task;
pub fn update(app:&mut App,event:Event)->Task<Message>{
 let current=app.app_providers.scope.as_ref().ok_or_else(||"无法连接网关".to_string()).and_then(|s|super::list::current(app,s.app));
 let generation=app.apps_catalog.generation;
 super::provider_update::run(app,|state,token,_|desktop_provider_apps::message::paths::update(state,current,token,generation,event))
}
use crate::app::components::system_settings_apps::local;
use vw_gateway_client::GatewayClient;
async fn pick_local(
    client: GatewayClient,
    scope: Scope,
    token: String,
) -> Result<Option<String>, ()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // A loopback URL alone does not prove this gateway belongs to the local user.
        let home = tokio::task::spawn_blocking(move || {
            local::launch_directory(&client, &scope.host, &token, None).ok_or(())
        })
        .await
        .map_err(|_| ())??;
        let selected = rfd::AsyncFileDialog::new().set_directory(home).pick_folder().await;
        selected.map(|file| file.path().to_str().map(str::to_owned).ok_or(())).transpose()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (client, scope, token);
        Err(())
    }
}

#[cfg(target_os = "macos")]
fn open_local(home: &str, path: &str) -> Result<(), ()> {
    use std::path::Path;
    let home = Path::new(home);
    let path = Path::new(path);
    let relative = path.strip_prefix(home).map_err(|_| ())?;
    if relative.as_os_str().is_empty()
        || !relative.components().all(|part| matches!(part, std::path::Component::Normal(_)))
    {
        return Err(());
    }
    let actual = path.canonicalize().map_err(|_| ())?;
    if actual != home.canonicalize().map_err(|_| ())?.join(relative) || !actual.is_dir() {
        return Err(());
    }
    let status = std::process::Command::new("/usr/bin/open")
        .args(["-a", "Finder"])
        .arg(actual)
        .status()
        .map_err(|_| ())?;
    status.success().then_some(()).ok_or(())
}

#[cfg(not(target_os = "macos"))]
fn open_local(_: &str, _: &str) -> Result<(), ()> {
    Err(())
}

pub fn message(event:Event)->Message {super::list::message(super::list::ListMessage::Paths(event))}

pub fn pick(client:GatewayClient,scope:Scope,token:String,generation:u64,catalog_generation:u64)->Task<Message>{
 Task::perform(pick_local(client,scope.clone(),token),move |result|message(Event::Picked{scope:scope.clone(),generation,catalog_generation,result}))
}
pub fn open(client:GatewayClient,scope:Scope,token:String,path:String)->Task<Message>{
 Task::perform(async move {tokio::task::spawn_blocking(move || {
 let home=local::launch_directory(&client,&scope.host,&token,None).ok_or(())?;
 open_local(&home,&path)
 }).await.ok().and_then(Result::ok)},|opened|if opened.is_some(){Message::None}else{message(Event::FolderFailed)})
}
