//! Production backend, filesystem skills and actual stdio MCP subprocesses.
use super::VibeWindowBackend;
use agent_client_protocol::{McpServer, McpServerStdio};
use serde_json::json;
use vw_acp::server::{BackendMountRequest, SessionBackend};
use vw_acp::{
    SessionMountRevision, SessionMountSelection, SessionSkillMount, SetSessionMountsRequest,
};

fn update(id: &str, revision: u64) -> SetSessionMountsRequest {
    SetSessionMountsRequest {
        session_id: id.to_string().into(),
        expected_revision: SessionMountRevision(revision),
        skills: vec![],
        mcp_servers: vec![],
        mcp_catalog: vec![],
    }
}
fn skill(root: &std::path::Path) -> SessionSkillMount {
    std::fs::create_dir_all(root).unwrap();
    std::fs::write(
        root.join("SKILL.md"),
        "---\nname: update_probe\ndescription: update probe\n---\nNative body\n",
    )
    .unwrap();
    SessionSkillMount { name: "update_probe".into(), path: root.to_str().unwrap().into() }
}
fn server(root: &std::path::Path, name: &str) -> McpServer {
    let script = root.join(format!("{name}.py"));
    std::fs::write(&script, r#"
import json, sys, os
from pathlib import Path
Path(sys.argv[1]).write_text(str(os.getpid()))
for line in sys.stdin:
    req = json.loads(line)
    method = req.get('method')
    if method == 'initialize':
        out = {'protocolVersion': req['params']['protocolVersion'], 'capabilities': {'tools': {}}, 'serverInfo': {'name': 'update', 'version': '1'}}
    elif method == 'tools/list':
        out = {'tools': [{'name': 'read', 'description': 'read file', 'inputSchema': {'type': 'object'}}]}
    elif method == 'tools/call':
        out = {'content': [{'type': 'text', 'text': Path(sys.argv[2]).read_text()}], 'isError': False}
    else:
        out = {}
    if 'id' in req:
        print(json.dumps({'jsonrpc': '2.0', 'id': req['id'], 'result': out}), flush=True)
"#).unwrap();
    std::fs::write(root.join(format!("{name}.txt")), name).unwrap();
    McpServer::Stdio(McpServerStdio::new(name, "python3").args(vec![
        script.to_str().unwrap().into(),
        root.join(format!("{name}.pid")).to_str().unwrap().into(),
        root.join(format!("{name}.txt")).to_str().unwrap().into(),
    ]))
}
async fn assert_exited(path: &std::path::Path) {
    let pid = std::fs::read_to_string(path).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if !std::process::Command::new("kill")
                .args(["-0", pid.trim()])
                .output()
                .unwrap()
                .status
                .success()
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("MCP child must exit");
}
#[tokio::test]
async fn acp_mount_update_rollback_replace_clear_and_revoke_real_processes() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let mount = skill(&source);
    let backend = VibeWindowBackend::default();
    let created = backend
        .create_session_with_mounts(
            root.path().into(),
            BackendMountRequest {
                skills: Some(SessionMountSelection { version: 1, skills: vec![mount.clone()] }),
                mcp_servers: vec![server(root.path(), "old")],
            },
        )
        .await
        .unwrap();
    let id = &created.session.id;
    let (old_skills, old_tools) = backend.mounted_runtime(id);
    let result = old_tools.tools()[0].call(json!({})).await.unwrap();
    assert!(result.is_success());
    assert!(result.model_text().contains("old"));
    let mut failed = update(id, 0);
    failed.mcp_servers = vec![
        server(root.path(), "candidate"),
        McpServer::Stdio(McpServerStdio::new("broken", "shiju-missing-command")),
    ];
    assert!(backend.set_session_mounts(failed).await.is_err());
    assert_exited(&root.path().join("candidate.pid")).await;
    assert!(old_tools.tools()[0].call(json!({})).await.unwrap().is_success());
    assert_eq!(backend.mounted_runtime(id).0.mounts(), &[mount]);
    let mut bad_skill = update(id, 0);
    bad_skill.skills = vec![SessionSkillMount {
        name: "missing".into(),
        path: root.path().join("missing").to_str().unwrap().into(),
    }];
    assert!(backend.set_session_mounts(bad_skill).await.is_err());
    assert!(old_skills.validate().is_ok());
    let mut replacement = update(id, 0);
    replacement.mcp_servers = vec![server(root.path(), "new")];
    let next_source = root.path().join("next-source");
    replacement.skills = vec![skill(&next_source)];
    std::fs::write(
        next_source.join("SKILL.md"),
        "---\nname: update_probe\ndescription: replacement\n---\nNew native body\n",
    )
    .unwrap();
    let response = backend.set_session_mounts(replacement).await.unwrap();
    assert_eq!(response.revision, SessionMountRevision(1));
    assert_eq!(response.mcp_servers, ["new"]);
    assert_eq!(response.skills, ["update_probe"]);
    assert!(old_skills.validate().is_err());
    assert_exited(&root.path().join("old.pid")).await;
    assert!(!old_tools.tools()[0].call(json!({})).await.is_ok_and(|r| r.is_success()));
    assert!(
        backend
            .set_session_mounts(update(id, 0))
            .await
            .unwrap_err()
            .to_string()
            .contains("conflict")
    );
    let (skills, tools) = backend.mounted_runtime(id);
    let catalog = crate::skills::load_session_skills(
        root.path(),
        &crate::config::Config::default(),
        &skills,
        true,
    )
    .unwrap();
    assert!(
        catalog
            .iter()
            .find(|s| s.name == "update_probe")
            .unwrap()
            .prompts
            .join("\n")
            .contains("New native body")
    );
    let result = tools.tools()[0].call(json!({})).await.unwrap();
    assert!(result.is_success());
    assert!(result.model_text().contains("new"));
    assert_eq!(
        backend.set_session_mounts(update(id, 1)).await.unwrap().revision,
        SessionMountRevision(2)
    );
    assert!(backend.mounted_runtime(id).1.is_empty());
    assert!(skills.validate().is_err());
    let loaded = backend
        .load_session_with_mounts(
            id,
            root.path().into(),
            BackendMountRequest { skills: None, mcp_servers: vec![] },
        )
        .await
        .unwrap();
    assert_eq!(loaded.mounts.revision, SessionMountRevision(2));
    assert!(loaded.mounts.skills.is_empty());
    assert!(backend.set_session_mounts(update(id, 0)).await.is_err());
    assert_exited(&root.path().join("new.pid")).await;
    assert!(!tools.tools()[0].call(json!({})).await.is_ok_and(|r| r.is_success()));
}
#[tokio::test]
async fn acp_mount_update_same_revision_has_one_winner() {
    let root = tempfile::tempdir().unwrap();
    let backend = VibeWindowBackend::default();
    let created = backend.create_session(root.path().into()).await.unwrap();
    let (a, b) = tokio::join!(
        backend.set_session_mounts(update(&created.id, 0)),
        backend.set_session_mounts(update(&created.id, 0))
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
}

#[tokio::test]
async fn acp_mount_update_protocol_blocks_prompt_load_and_second_update_during_prepare() {
    use agent_client_protocol::{
        Agent, ContentBlock, InitializeRequest, LoadSessionRequest, NewSessionRequest,
        PromptRequest, ProtocolVersion, TextContent,
    };
    let root = tempfile::tempdir().unwrap();
    let mcp = server(root.path(), "slow");
    let script = root.path().join("slow.py");
    let text = std::fs::read_to_string(&script).unwrap().replace("out = {'tools':", "import time\n        Path(sys.argv[1] + '.preparing').touch()\n        while not Path(sys.argv[1] + '.release').exists(): time.sleep(0.01)\n        out = {'tools':");
    std::fs::write(&script, text).unwrap();
    let agent = vw_acp::server::VibeWindowAgent::new(VibeWindowBackend::default());
    agent.initialize(InitializeRequest::new(ProtocolVersion::V1)).await.unwrap();
    let created = agent.new_session(NewSessionRequest::new(root.path())).await.unwrap();
    let id = created.session_id.to_string();
    let mut request = update(&id, 0);
    request.mcp_servers = vec![mcp];
    let capabilities =
        vw_acp::SessionMountCapabilities { version: 1, skills: true, set_mounts: true };
    let ext = request.to_ext_request(Some(&capabilities)).unwrap();
    let busy = async {
        while !root.path().join("slow.pid.preparing").exists() {
            tokio::task::yield_now().await;
        }
        let prompt = agent
            .prompt(PromptRequest::new(
                id.clone(),
                vec![ContentBlock::Text(TextContent::new("must not start"))],
            ))
            .await;
        assert!(prompt.unwrap_err().to_string().contains("active"));
        let load = agent.load_session(LoadSessionRequest::new(id.clone(), root.path())).await;
        assert!(load.unwrap_err().to_string().contains("busy"));
        let second =
            agent.ext_method(update(&id, 0).to_ext_request(Some(&capabilities)).unwrap()).await;
        assert!(second.unwrap_err().to_string().contains("busy"));
        std::fs::write(root.path().join("slow.pid.release"), "").unwrap();
    };
    tokio::time::timeout(std::time::Duration::from_secs(15), async {
        let (result, ()) = tokio::join!(agent.ext_method(ext), busy);
        let result: vw_acp::SetSessionMountsResponse =
            serde_json::from_str(result.unwrap().0.get()).unwrap();
        assert_eq!(result.revision, SessionMountRevision(1));
    })
    .await
    .unwrap();
    agent.ext_method(update(&id, 1).to_ext_request(Some(&capabilities)).unwrap()).await.unwrap();
    assert_exited(&root.path().join("slow.pid")).await;
}
