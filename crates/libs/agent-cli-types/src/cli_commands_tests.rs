// 本文件验证命令行服务与渠道命令的稳定序列化及值语义。

use super::cli_commands::{
    ChannelCommands, CronCommands, IntegrationCommands, MemoryCommands, MigrateCommands,
    ServiceCommands, SkillCommands,
};

#[test]
// 命令变体名称属于 CLI 契约，序列化结果必须稳定。
fn service_commands_serialize_to_stable_variant_names() {
    assert_eq!(serde_json::to_value(ServiceCommands::Status).unwrap(), "Status");
}

#[test]
fn channel_add_preserves_type_and_json_config() {
    let command = ChannelCommands::Add {
        channel_type: "telegram".to_string(),
        config: "{\"bot_token\":\"x\"}".to_string(),
    };

    let round_trip: ChannelCommands =
        serde_json::from_value(serde_json::to_value(&command).unwrap()).unwrap();

    assert_eq!(round_trip, command);
}

#[test]
fn command_enums_are_cloneable_and_comparable() {
    assert_eq!(SkillCommands::List.clone(), SkillCommands::List);
    assert_eq!(
        MigrateCommands::Openclaw { source: None, dry_run: true }.clone(),
        MigrateCommands::Openclaw { source: None, dry_run: true }
    );
    assert_eq!(CronCommands::List.clone(), CronCommands::List);
    assert_eq!(
        MemoryCommands::List { category: None, session: None, limit: 50, offset: 0 }.clone(),
        MemoryCommands::List { category: None, session: None, limit: 50, offset: 0 }
    );
    assert_eq!(
        IntegrationCommands::List { category: None, status: None }.clone(),
        IntegrationCommands::List { category: None, status: None }
    );
}
