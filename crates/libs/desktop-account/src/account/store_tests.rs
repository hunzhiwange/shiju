//! 验证多账户仓储的持久化、激活切换、兼容读取与敏感信息保护行为。
//! 临时目录中的真实文件还用于检查旧格式迁移、无效活动记录和 Unix 权限约束。

use super::*;
use std::fs;

// 两组构造器分别表示旧版最小账户与带完整资料的新格式账户，供迁移测试复用。
fn verified(skey: &str, company_id: &str, company_name: &str) -> VerifiedAccount {
    VerifiedAccount::from_parts(
        AccountIdentity::new(company_id, company_name, "founder", "active").unwrap(),
        SecretSkey::new(skey).unwrap(),
    )
}

fn verified_with_profile(skey: &str, company_name: &str) -> VerifiedAccount {
    VerifiedAccount::from_parts_with_profile(
        AccountIdentity::from_profile(company_name, "enabled").unwrap(),
        AccountProfile::new(
            "enabled",
            "刘祥敏",
            "liuxiangmin",
            "enabled",
            vec!["AI创新组".to_string()],
            vec!["默认角色".to_string(), "数据管理员".to_string()],
        )
        .unwrap(),
        SecretSkey::new(skey).unwrap(),
    )
}

#[test]
fn repository_upserts_same_skey_without_duplicates() {
    let mut repository = AccountRepository::default();
    let first = repository
        .upsert_verified(verified("same-key", "company-1", "旧名称"), AccountOrigin::ManualSkey);
    let second = repository
        .upsert_verified(verified("same-key", "company-1", "新名称"), AccountOrigin::BrowserLogin);

    assert_eq!(first, second);
    assert_eq!(repository.accounts().len(), 1);
    assert_eq!(repository.active_account_id(), Some(first.as_str()));
    assert_eq!(repository.active_account().unwrap().identity().company_name(), "新名称");
    assert_eq!(repository.active_account().unwrap().origin(), AccountOrigin::BrowserLogin);
}

#[test]
fn repository_switches_exactly_one_active_account() {
    let mut repository = AccountRepository::default();
    let first = repository
        .upsert_verified(verified("key-1", "company-1", "一号"), AccountOrigin::ManualSkey);
    let second = repository
        .upsert_verified(verified("key-2", "company-2", "二号"), AccountOrigin::BrowserLogin);

    repository.activate(&first).unwrap();
    assert_eq!(repository.active_account_id(), Some(first.as_str()));
    assert_eq!(repository.active_account().unwrap().id(), first);
    repository.activate(&second).unwrap();
    assert_eq!(repository.active_account_id(), Some(second.as_str()));
    assert_eq!(repository.active_account().unwrap().id(), second);
    assert!(repository.activate("missing").is_err());
}

#[test]
fn repository_sign_out_retains_all_accounts() {
    let mut repository = AccountRepository::default();
    repository.upsert_verified(verified("key-1", "company-1", "一号"), AccountOrigin::ManualSkey);
    repository.upsert_verified(verified("key-2", "company-2", "二号"), AccountOrigin::BrowserLogin);
    repository.sign_out();

    assert_eq!(repository.accounts().len(), 2);
    assert!(repository.active_account_id().is_none());
    assert!(repository.active_account().is_none());
}

#[test]
fn repository_removes_saved_accounts_and_clears_deleted_active_account() {
    let mut repository = AccountRepository::default();
    let inactive = repository
        .upsert_verified(verified("key-1", "company-1", "一号"), AccountOrigin::ManualSkey);
    let active = repository
        .upsert_verified(verified("key-2", "company-2", "二号"), AccountOrigin::BrowserLogin);

    repository.remove(&inactive).unwrap();
    assert_eq!(repository.accounts().len(), 1);
    assert_eq!(repository.active_account_id(), Some(active.as_str()));

    repository.remove(&active).unwrap();
    assert!(repository.accounts().is_empty());
    assert!(repository.active_account_id().is_none());
    assert!(repository.remove("missing").is_err());
}

#[test]
fn store_round_trips_multiple_accounts() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("shiju-accounts.json");
    let mut repository = AccountRepository::default();
    repository.upsert_verified(verified("key-1", "company-1", "一号"), AccountOrigin::ManualSkey);
    let active = repository
        .upsert_verified(verified_with_profile("key-2", "二号"), AccountOrigin::BrowserLogin);
    repository.save_to(&path).unwrap();

    let loaded = AccountRepository::load_from(&path).unwrap();
    assert_eq!(loaded.accounts().len(), 2);
    assert_eq!(loaded.active_account_id(), Some(active.as_str()));
    assert_eq!(loaded.active_account().unwrap().skey().expose(), "key-2");
    let profile = loaded.active_account().unwrap().profile().expect("stored profile");
    assert_eq!(profile.name(), "刘祥敏");
    assert_eq!(profile.departments(), ["AI创新组"]);
    assert_eq!(profile.roles(), ["默认角色", "数据管理员"]);
}

#[test]
fn stored_account_skill_hub_credential_remains_redacted() {
    let mut repository = AccountRepository::default();
    repository.upsert_verified(
        verified("catalog-secret", "company-1", "示例公司"),
        AccountOrigin::BrowserLogin,
    );

    let credential = repository.active_account().unwrap().skill_hub_credential();

    assert_eq!(credential.expose(), "catalog-secret");
    assert_eq!(format!("{credential:?}"), "<redacted>");
    assert!(!format!("{:?}", repository.active_account().unwrap()).contains("catalog-secret"));
}

#[test]
fn account_profile_edit_refreshes_and_persists_current_account() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("shiju-accounts.json");
    let mut repository = AccountRepository::default();
    let account_id = repository.upsert_verified(
        verified_with_profile("profile-edit-secret", "示例公司"),
        AccountOrigin::BrowserLogin,
    );

    repository
        .refresh_active_profile(vw_gateway_client::ShijuAccountMeDto {
            user_id: "101026".to_string(),
            company_id: "10011".to_string(),
            account_id: "101026".to_string(),
            account_name: "liuxiangmin2".to_string(),
            name: "刘祥敏2".to_string(),
            source_type: "manual".to_string(),
            external_user_id: String::new(),
            allow_console_login: true,
            status: "enabled".to_string(),
            departments: vec![vw_gateway_client::ShijuAccountDepartmentDto {
                department_id: "101014".to_string(),
                department_name: "AI创新组".to_string(),
                parent_department_id: "101005".to_string(),
            }],
            roles: vec![vw_gateway_client::ShijuAccountRoleDto {
                role_id: "102".to_string(),
                role_name: "默认角色".to_string(),
                role_code: "default".to_string(),
            }],
        })
        .unwrap();
    repository.save_to(&path).unwrap();

    let loaded = AccountRepository::load_from(&path).unwrap();
    let account = loaded.active_account().expect("active account");
    let profile = account.profile().expect("profile");
    assert_eq!(account.id(), account_id);
    assert_eq!(account.origin(), AccountOrigin::BrowserLogin);
    assert_eq!(account.skey().expose(), "profile-edit-secret");
    assert_eq!(account.identity().company_name(), "示例公司");
    assert_eq!(profile.name(), "刘祥敏2");
    assert_eq!(profile.account_name(), "liuxiangmin2");
    assert_eq!(profile.departments(), ["AI创新组"]);
    assert_eq!(profile.roles(), ["默认角色"]);
}

#[test]
fn store_loads_legacy_accounts_without_profile() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("shiju-accounts.json");
    let skey = SecretSkey::new("legacy-key").unwrap();
    let id = account_id(&skey);
    fs::write(
        &path,
        format!(
            r#"{{
              "accounts": [{{
                "id":"{id}",
                "company_id":"company-1",
                "company_name":"旧账户",
                "founder_user_id":"founder",
                "status":"active",
                "origin":"manual_skey",
                "skey":"legacy-key"
              }}],
              "active_account_id":"{id}"
            }}"#
        ),
    )
    .unwrap();

    let loaded = AccountRepository::load_from(&path).unwrap();

    assert!(loaded.active_account().unwrap().profile().is_none());
}

#[test]
fn store_rejects_duplicate_or_dangling_active_records() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("shiju-accounts.json");
    let duplicate = r#"{
      "accounts": [
        {"id":"x","company_id":"c","company_name":"n","founder_user_id":"f","status":"active","origin":"manual_skey","skey":"key"},
        {"id":"x","company_id":"c","company_name":"n","founder_user_id":"f","status":"active","origin":"manual_skey","skey":"key"}
      ],
      "active_account_id":"x"
    }"#;
    fs::write(&path, duplicate).unwrap();
    assert!(AccountRepository::load_from(&path).is_err());

    fs::write(&path, r#"{"accounts":[],"active_account_id":"missing"}"#).unwrap();
    assert!(AccountRepository::load_from(&path).is_err());
    fs::write(&path, "[]").unwrap();
    assert!(AccountRepository::load_from(&path).is_err());
}

#[cfg(unix)]
#[test]
fn store_uses_owner_only_permissions_on_unix() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("shiju-accounts.json");
    AccountRepository::default().save_to(&path).unwrap();
    assert_eq!(fs::metadata(path).unwrap().permissions().mode() & 0o777, 0o600);
}
