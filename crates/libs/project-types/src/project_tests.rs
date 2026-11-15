//! 项目信息模型的完整 JSON 往返测试。
//!
//! 夹具覆盖版本控制、图标、启动命令、时间信息与沙箱路径等可选结构。

use super::{Commands, Icon, Info, TimeInfo, Vcs};

#[test]
fn project_info_full_model_round_trips_through_json() {
    let expected = Info {
        id: "project-alpha".to_owned(),
        worktree: "/workspace/alpha".to_owned(),
        vcs: Some(Vcs::Git),
        name: Some("Alpha".to_owned()),
        icon: Some(Icon {
            url: Some("https://example.test/alpha.png".to_owned()),
            override_icon: Some("rocket".to_owned()),
            color: Some("#123456".to_owned()),
        }),
        commands: Some(Commands { start: Some("cargo run".to_owned()) }),
        time: TimeInfo { created: 10, updated: 20, initialized: Some(15) },
        sandboxes: vec!["/workspace/alpha-sandbox".to_owned()],
    };

    let json = serde_json::to_string(&expected).expect("project info should serialize");
    let actual: Info = serde_json::from_str(&json).expect("project info should deserialize");

    assert_eq!(actual, expected);
}
