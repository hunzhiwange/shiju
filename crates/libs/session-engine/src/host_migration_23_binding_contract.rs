//! Fork selection acceptance driven by the real admission and lifecycle host.
use crate::{agent_binding::AgentBinding,admission_host::AdmissionHost};
pub trait BindingBoundaryHost: AdmissionHost {
    fn create_next(input:crate::session_io::CreateInput)->impl Future<Output=Result<session_info::Info,session_info::Error>> + Send;
    fn create_fork(project:&str,session:&str,ids:Vec<String>)->impl Future<Output=anyhow::Result<()>> + Send;
    fn start_fork(project:&str,session:&str)->impl Future<Output=anyhow::Result<()>> + Send;
}
pub async fn agent_binding_inherited_history_cannot_override_fork_selection<H:BindingBoundaryHost>() {
            let info = H::create_next(crate::session_io::CreateInput {
                directory: agent_project::instance::directory(),
                ..Default::default()
            })
            .await
            .unwrap();
            let user: crate::message::Info = serde_json::from_value(serde_json::json!({
                "role":"user", "id":"inherited-user", "sessionID":info.id,
                "time":{"created":1}, "agent":"acp-claude",
                "model":{"providerID":"model-recovery","modelID":"original"}
            }))
            .unwrap();
            crate::message::write_message(&user).await.unwrap();
            let inherited = AgentBinding::from_definition(
                "acp-claude".into(),
                &H::agent("acp-claude").await.unwrap(),
            );
            H::save_binding(&info.project_id, &info.id, &inherited).await.unwrap();
            H::create_fork(
                &info.project_id,
                &info.id,
                vec!["inherited-user".into()],
            )
            .await
            .unwrap();
            let path = std::path::PathBuf::from(&info.directory);
            for (agent, backend) in
                [("main", None), ("acp-codex", Some("codex")), ("acp-claude", Some("claude"))]
            {
                let (binding, _) =
                    crate::admission::prepare::<H>(&info.id, Some(agent), backend, Some("model-recovery/original"), &path)
                        .await
                        .unwrap();
                assert_eq!(binding.agent, agent);
            }
            H::start_fork(&info.project_id, &info.id).await.unwrap();
            assert!(
                crate::admission::prepare::<H>(&info.id, Some("main"), None, Some("model-recovery/original"), &path)
                    .await
                    .err()
                    .expect("started fork rejects a new agent")
                    .to_string()
                    .contains("new session")
            );
            assert_eq!(H::load_binding(&info.id).await.unwrap().unwrap().agent, "acp-claude");
}
