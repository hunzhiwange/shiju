// 验证 Shiju 应用加载、账号结果过滤与绑定后启动流程。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use crate::app::account::{
    AccountIdentity, AccountOrigin, AccountRepository, SecretSkey, VerifiedAccount,
};
use crate::app::message::ShijuAppsMessage;
use crate::app::shiju_apps::{ShijuAppsLoadState, ShijuAppsTab};
use crate::app::{App, Message, Screen};

fn repository(secret: &str, company_id: &str) -> AccountRepository {
    let mut repository = AccountRepository::default();
    repository.upsert_verified(
        VerifiedAccount::from_parts(
            AccountIdentity::new(company_id, "示例公司", "founder", "active").unwrap(),
            SecretSkey::new(secret).unwrap(),
        ),
        AccountOrigin::BrowserLogin,
    );
    repository
}

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
#[test]
fn shiju_apps_open_loads_mine_through_gateway() {
    let _ = desktop_session::session_gateway::gateway_shiju_app_catalog_async;
    let _ = desktop_session::session_gateway::gateway_shiju_apps_async;
    let _ = desktop_session::session_gateway::gateway_shiju_app_prepare_launch_async;
    let (mut app, _) = App::new();
    app.account.replace_repository(repository("account-secret", "company-1"));

    let _task = app.update(Message::ShijuApps(ShijuAppsMessage::Open));

    assert_eq!(app.navigation_tabs.screen, Screen::ShijuApps);
    assert_eq!(app.shiju_apps.tab, ShijuAppsTab::Mine);
    assert_eq!(app.shiju_apps.account_id.as_deref(), Some("company-1"));
    assert!(matches!(app.shiju_apps.mine, ShijuAppsLoadState::Loading { .. }));
    let tabs = app.navigation_tabs.open_tabs.iter().filter(|tab| tab.id == "shiju_apps").collect::<Vec<_>>();
    assert_eq!(tabs.len(), 1);
    assert_eq!(tabs[0].title, "识矩应用");
    assert_eq!(tabs[0].screen, Screen::ShijuApps);
}
