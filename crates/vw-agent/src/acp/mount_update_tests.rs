//! Real native backend ports for the ACP update contract suite.
use super::backend::NativeAcpHost;
use agent_acp::host_migration_53_update_contracts as contracts;

#[tokio::test]
async fn acp_mount_update_rollback_replace_clear_and_revoke_real_processes() {
    contracts::acp_mount_update_rollback_replace_clear_and_revoke_real_processes::<NativeAcpHost>(|root, skills| {
        let catalog = crate::skills::load_session_skills(root, &crate::config::Config::default(), skills, true).unwrap();
        catalog.iter().find(|skill| skill.name == "update_probe").unwrap().prompts.join("\n")
    }).await;
}

#[tokio::test]
async fn acp_mount_update_same_revision_has_one_winner() {
    contracts::acp_mount_update_same_revision_has_one_winner::<NativeAcpHost>().await;
}

#[tokio::test]
async fn acp_mount_update_protocol_blocks_prompt_load_and_second_update_during_prepare() {
    contracts::acp_mount_update_protocol_blocks_prompt_load_and_second_update_during_prepare::<NativeAcpHost>().await;
}
