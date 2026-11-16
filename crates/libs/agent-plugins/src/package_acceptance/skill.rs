use super::{digest, public_original};
use crate::{install_plugin_from_marketplace, prepare_plugin_upgrade};
use std::{path::Path, future::Future};
/// Run the original package lifecycle with the application's real skill invocation.
pub async fn lifecycle<F, Fut>(fixtures: &Path, invoke: F,
    load_skills: &dyn Fn(&Path, &agent_config::runtime::Config) -> Vec<agent_skills::Skill>,
    close_plugin: &dyn Fn(&Path))
where F: FnOnce(agent_config::runtime::Config, String) -> Fut,
      Fut: Future<Output = anyhow::Result<tool_types::ToolCallResult>> {
    let name = "superpowers:verification-before-completion";
    let t = tempfile::tempdir().unwrap();
    let source = public_original(fixtures, t.path(), "superpowers-6.3.0");
    let upgrade = public_original(fixtures, t.path(), "superpowers-7.0.1");
    let before = digest(&source);
    let next = digest(&upgrade);
    let mut config = agent_config::runtime::Config::default();
    config.workspace_dir = t.path().join("workspace");
    config.config_path = t.path().join("config/shiju.json");
    let plugins = t.path().join("config/plugins");
    let installed =
        install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "public").unwrap();
    assert_eq!(digest(&installed.installed_dir), before);
    let result = invoke(config.clone(), name.to_owned()).await.unwrap();
    assert_eq!(result.data["success"], true, "{result:?}");
    assert!(result.extra_messages.iter().any(|m| {
        m.content.to_string().contains("NO COMPLETION CLAIMS WITHOUT FRESH VERIFICATION EVIDENCE")
    }));
    crate::install::set_plugin_enabled(&plugins, "superpowers@public", false, close_plugin).unwrap();
    assert!(
        load_skills(&config.workspace_dir, &config)
            .iter()
            .all(|s| s.name != name)
    );
    crate::install::set_plugin_enabled(&plugins, "superpowers@public", true, close_plugin).unwrap();
    let preview =
        prepare_plugin_upgrade(upgrade.to_str().unwrap(), &plugins, "superpowers@public").unwrap();
    crate::upgrade::activate_plugin_upgrade(preview, close_plugin).unwrap();
    assert_eq!(digest(&installed.installed_dir), next);
    crate::install::remove_installed_plugin(&plugins, "superpowers@public", close_plugin).unwrap();
    assert!(!installed.installed_dir.exists());
    assert_eq!(digest(&source), before);
    assert_eq!(digest(&upgrade), next);
}
