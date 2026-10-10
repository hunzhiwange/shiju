#![allow(unused_imports, dead_code)]

// 验证启用频道汇总、配置构建与持久化前置校验。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;
use crate::app::message::settings::channels::toggles;
use vw_config_types::channels::{
    DingTalkConfig, DingTalkInstanceConfig, FeishuConfig, FeishuInstanceConfig,
};

fn app() -> App {
    App::new().0
}




mod channel_instance_settings {
    use super::*;


}
