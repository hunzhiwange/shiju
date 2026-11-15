pub use desktop_provider_apps::message::edit::{EditMessage,save};
use super::{agy,claude,codex,dsh,gemini,grok,hermes,kimi,mcode,omp,openclaw,opencode,pi};
use crate::app::{App,Message,message::SettingsMessage};
use iced::Task;
use vw_api_types::agent_apps::AppId;
pub(crate) fn message(value: EditMessage)->Message { Message::Settings(SettingsMessage::AppProviderEdit(value)) }
pub(crate) fn update(app:&mut App,event:EditMessage)->Task<Message> {
 if matches!(&event, EditMessage::Claude(_) | EditMessage::Codex(_) | EditMessage::Gemini(_) | EditMessage::Grok(_) | EditMessage::Hermes(_) | EditMessage::Pi(_) | EditMessage::Kimi(_) | EditMessage::Mcode(_) | EditMessage::Agy(_) | EditMessage::Dsh(_) | EditMessage::Omp(_) | EditMessage::OpenClaw(_) | EditMessage::OpenCode(_)) {
  return super::provider_update::effect(app,desktop_provider_apps::message::provider_update::Effect::Field(event));
 }
 let current=app.app_providers.scope.as_ref().and_then(|s|super::list::current(app,s.app).ok());
 super::provider_update::edit(app,current,event)
}
pub(crate) fn load(client:vw_gateway_client::GatewayClient,scope:desktop_provider_apps::Scope,generation:u64,id:String,copy:bool,delete:bool,claude:bool,token:String)->Task<Message> {
            let source = id.clone();
            if claude && !copy && !delete {
                let token = token.clone();
                let request_scope = scope.clone();
                return Task::perform(
                    async move { claude::load(client, request_scope, &source, &token).await },
                    move |result| {
                        message(EditMessage::ClaudeLoaded {
                            scope: scope.clone(),
                            generation,
                            id: id.clone(),
                            result,
                        })
                    },
                );
            }
            if scope.app == AppId::Codex && !copy && !delete {
                let token = token.clone();
                let request_scope = scope.clone();
                return Task::perform(
                    async move { codex::load(client, request_scope, &source, &token).await },
                    move |result| {
                        message(EditMessage::CodexLoaded {
                            scope: scope.clone(),
                            generation,
                            id: id.clone(),
                            result,
                        })
                    },
                );
            }
            if scope.app == AppId::Gemini && !copy && !delete {
                let token = token.clone();
                let request_scope = scope.clone();
                return Task::perform(
                    async move { gemini::load(client, request_scope, &source, &token).await },
                    move |result| {
                        message(EditMessage::GeminiLoaded {
                            scope: scope.clone(),
                            generation,
                            id: id.clone(),
                            result,
                        })
                    },
                );
            }
            if scope.app == AppId::GrokBuild && !copy && !delete {
                let token = token.clone();
                let request_scope = scope.clone();
                return Task::perform(
                    async move { grok::load(client, request_scope, &source, &token).await },
                    move |result| {
                        message(EditMessage::GrokLoaded {
                            scope: scope.clone(),
                            generation,
                            id: id.clone(),
                            result,
                        })
                    },
                );
            }
            if scope.app == AppId::Hermes && !copy && !delete {
                let token = token.clone();
                let request_scope = scope.clone();
                return Task::perform(
                    async move { hermes::load(client, request_scope, &source, &token).await },
                    move |result| {
                        message(EditMessage::HermesLoaded {
                            scope: scope.clone(),
                            generation,
                            id: id.clone(),
                            result,
                        })
                    },
                );
            }
            if scope.app == AppId::Pi && !copy && !delete {
                let token = token.clone();
                let request_scope = scope.clone();
                return Task::perform(
                    async move { pi::load(client, request_scope, &source, &token).await },
                    move |result| {
                        message(EditMessage::PiLoaded {
                            scope: scope.clone(),
                            generation,
                            id: id.clone(),
                            result,
                        })
                    },
                );
            }
            if scope.app == AppId::Kimi && !copy && !delete {
                let token = token.clone();
                let request_scope = scope.clone();
                return Task::perform(
                    async move { kimi::load(client, request_scope, &source, &token).await },
                    move |result| {
                        message(EditMessage::KimiLoaded {
                            scope: scope.clone(),
                            generation,
                            id: id.clone(),
                            result,
                        })
                    },
                );
            }
            if scope.app == AppId::Mcode && !copy && !delete {
                let token = token.clone();
                let request_scope = scope.clone();
                return Task::perform(
                    async move { mcode::load(client, request_scope, &source, &token).await },
                    move |result| {
                        message(EditMessage::McodeLoaded {
                            scope: scope.clone(),
                            generation,
                            id: id.clone(),
                            result,
                        })
                    },
                );
            }
            if scope.app == AppId::Agy && !copy && !delete {
                let token = token.clone();
                let request_scope = scope.clone();
                return Task::perform(
                    async move { agy::load(client, request_scope, &source, &token).await },
                    move |result| {
                        message(EditMessage::AgyLoaded {
                            scope: scope.clone(),
                            generation,
                            id: id.clone(),
                            result,
                        })
                    },
                );
            }
            if scope.app == AppId::Dsh && !copy && !delete {
                let token = token.clone();
                let request_scope = scope.clone();
                return Task::perform(
                    async move { dsh::load(client, request_scope, &source, &token).await },
                    move |result| {
                        message(EditMessage::DshLoaded {
                            scope: scope.clone(),
                            generation,
                            id: id.clone(),
                            result,
                        })
                    },
                );
            }
            if scope.app == AppId::Omp && !copy && !delete {
                let token = token.clone();
                let request_scope = scope.clone();
                return Task::perform(
                    async move { omp::load(client, request_scope, &source, &token).await },
                    move |result| {
                        message(EditMessage::OmpLoaded {
                            scope: scope.clone(),
                            generation,
                            id: id.clone(),
                            result,
                        })
                    },
                );
            }
            if scope.app == AppId::OpenClaw && !copy && !delete {
                let token = token.clone();
                let request_scope = scope.clone();
                return Task::perform(
                    async move { openclaw::load(client, request_scope, &source, &token).await },
                    move |result| {
                        message(EditMessage::OpenClawLoaded {
                            scope: scope.clone(),
                            generation,
                            id: id.clone(),
                            result,
                        })
                    },
                );
            }
            if scope.app == AppId::OpenCode && !copy && !delete {
                let token = token.clone();
                let request_scope = scope.clone();
                return Task::perform(
                    async move { opencode::load(client, request_scope, &source, &token).await },
                    move |result| {
                        message(EditMessage::OpenCodeLoaded {
                            scope: scope.clone(),
                            generation,
                            id: id.clone(),
                            result,
                        })
                    },
                );
            }
            return Task::perform(
                async move { gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone()).app_provider_basic(scope.app, &source).await },
                move |result| {
                    message(EditMessage::Loaded {
                        scope: scope.clone(),
                        generation,
                        id: id.clone(),
                        copy,
                        delete,
                        result,
                    })
                },
            );
}
pub(crate) fn dispatch_save(client:vw_gateway_client::GatewayClient,scope:desktop_provider_apps::Scope,generation:u64,form:desktop_provider_apps::state::edit::EditForm,token:String)->Task<Message> {
            let claude = form.claude_editable();
            let gemini = form.gemini_editable();
            let codex = form.codex.is_some();
            let grok = form.grok.is_some();
            let opencode = form.opencode.is_some();
            let hermes = form.hermes.is_some();
            let pi = form.pi.is_some();
            let kimi = form.kimi.is_some();
            let mcode = form.mcode.is_some();
            let agy = form.agy.is_some();
            let dsh = form.dsh.is_some();
            let omp = form.omp.is_some();
            let openclaw = form.openclaw.is_some();
            if claude {
                return Task::perform(
                    claude::save(client, scope.clone(), form, token),
                    move |result| {
                        message(EditMessage::ClaudeSaved {
                            scope: scope.clone(),
                            generation,
                            result,
                        })
                    },
                );
            }
            if codex {
                return Task::perform(
                    codex::save(client, scope.clone(), form, token),
                    move |result| {
                        message(EditMessage::CodexSaved {
                            scope: scope.clone(),
                            generation,
                            result,
                        })
                    },
                );
            }
            if gemini {
                return Task::perform(
                    gemini::save(client, scope.clone(), form, token),
                    move |result| {
                        message(EditMessage::GeminiSaved {
                            scope: scope.clone(),
                            generation,
                            result,
                        })
                    },
                );
            }
            if grok {
                return Task::perform(
                    grok::save(client, scope.clone(), form, token),
                    move |result| {
                        message(EditMessage::GrokSaved { scope: scope.clone(), generation, result })
                    },
                );
            }
            if hermes {
                return Task::perform(
                    hermes::save(client, scope.clone(), form, token),
                    move |result| {
                        message(EditMessage::HermesSaved {
                            scope: scope.clone(),
                            generation,
                            result,
                        })
                    },
                );
            }
            if pi {
                return Task::perform(
                    pi::save(client, scope.clone(), form, token),
                    move |result| {
                        message(EditMessage::PiSaved { scope: scope.clone(), generation, result })
                    },
                );
            }
            if kimi {
                return Task::perform(
                    kimi::save(client, scope.clone(), form, token),
                    move |result| {
                        message(EditMessage::KimiSaved { scope: scope.clone(), generation, result })
                    },
                );
            }
            if mcode {
                return Task::perform(
                    mcode::save(client, scope.clone(), form, token),
                    move |result| {
                        message(EditMessage::McodeSaved {
                            scope: scope.clone(),
                            generation,
                            result,
                        })
                    },
                );
            }
            if agy {
                return Task::perform(
                    agy::save(client, scope.clone(), form, token),
                    move |result| {
                        message(EditMessage::AgySaved { scope: scope.clone(), generation, result })
                    },
                );
            }
            if dsh {
                return Task::perform(
                    dsh::save(client, scope.clone(), form, token),
                    move |result| {
                        message(EditMessage::DshSaved { scope: scope.clone(), generation, result })
                    },
                );
            }
            if omp {
                return Task::perform(
                    omp::save(client, scope.clone(), form, token),
                    move |result| {
                        message(EditMessage::OmpSaved { scope: scope.clone(), generation, result })
                    },
                );
            }
            if openclaw {
                return Task::perform(
                    openclaw::save(client, scope.clone(), form, token),
                    move |result| {
                        message(EditMessage::OpenClawSaved {
                            scope: scope.clone(),
                            generation,
                            result,
                        })
                    },
                );
            }
            if opencode {
                return Task::perform(
                    opencode::save(client, scope.clone(), form, token),
                    move |result| {
                        message(EditMessage::OpenCodeSaved {
                            scope: scope.clone(),
                            generation,
                            result,
                        })
                    },
                );
            }
            return Task::perform(save(client, scope.clone(), form, token), move |result| {
                message(EditMessage::Saved { scope: scope.clone(), generation, result })
            });
}
