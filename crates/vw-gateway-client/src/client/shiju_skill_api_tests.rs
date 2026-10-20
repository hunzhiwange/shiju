//! 验证识矩私有技能的查询、详情、版本和安装接口。
//!
//! 用例覆盖当前与历史响应形状、可选展示元数据，并确保凭据不进入序列化请求体。

use serde_json::json;

use super::{
    SHIJU_CREDENTIAL_HEADER, ShijuCredential, ShijuMaintainedSkillQuery, ShijuSkillCursorPageDto,
    ShijuSkillDetailDto, ShijuSkillPageDto, ShijuSkillReadmeDto, ShijuSkillSearchQuery,
    ShijuSkillVersionDto, ShijuSystemSkillQuery,
};
use crate::client::test_support;

fn credential() -> ShijuCredential {
    ShijuCredential::new("fixture-private-skill-secret").expect("credential")
}

#[tokio::test]
async fn shiju_skill_client_uses_private_gateway_paths() {
    let server = test_support::server(vec![
        (200, json!({"items": [], "next_cursor": null, "has_more": false})),
        (200, json!([])),
        (200, json!({"items": [], "page": 2, "page_size": 20, "total": 0})),
        (200, json!({"items": [], "page": 4, "page_size": 20, "total": 0})),
        (
            200,
            json!({
                "id": 31,
                "slug": "shiju-visual",
                "name": "识矩引擎视觉",
                "description": "品牌视觉规范",
                "level": "company",
                "modality": "local",
                "format": "zip",
                "company_id": "10011",
                "created_uid": "101027",
                "maintainer_uid": "101027",
                "status": "published",
                "is_all_company_accessible": 0,
                "current_version_id": 60,
                "current_version": null,
                "created_at": "2026-07-28T09:55:24+08:00",
                "updated_at": "2026-07-28T09:56:15+08:00",
                "tags": []
            }),
        ),
        (200, json!({"markdown": "# 使用说明\n支持 UTF-8 正文。"})),
        (200, json!([])),
    ]);
    let credential = credential();

    let search = ShijuSkillSearchQuery {
        query: Some("visual guide".to_string()),
        cursor: Some("cursor/next".to_string()),
        page_size: 20,
        tag_ids: Some(2),
        company_tag_ids: Some(5),
    };
    let system = ShijuSystemSkillQuery {
        page: 2,
        page_size: 20,
        name: Some("system visual".to_string()),
        tag_id: Some(7),
    };
    let maintained = ShijuMaintainedSkillQuery {
        page: 4,
        page_size: 20,
        keyword: Some("my visual".to_string()),
        tag_ids: vec![2, 1],
        company_tag_ids: vec![5],
        statuses: vec![
            "draft".to_string(),
            "pending_review".to_string(),
            "published".to_string(),
            "unpublished".to_string(),
        ],
    };

    server.client().shiju_skill_search(&credential, &search).await.unwrap();
    server.client().shiju_skill_installed(&credential).await.unwrap();
    server.client().shiju_system_skills(&credential, &system).await.unwrap();
    server.client().shiju_maintained_skills(&credential, &maintained).await.unwrap();
    server.client().shiju_skill_detail(&credential, 31).await.unwrap();
    let readme = server.client().shiju_skill_readme(&credential, 31).await.unwrap();
    server.client().shiju_skill_versions(&credential, 31).await.unwrap();
    assert_eq!(
        readme,
        ShijuSkillReadmeDto {
            markdown: Some("# 使用说明\n支持 UTF-8 正文。".to_string())
        }
    );

    let expected_paths = [
        "/v1/shiju/skills/search?page_size=20&q=visual+guide&cursor=cursor%2Fnext&tag_ids=2&company_tag_ids=5",
        "/v1/shiju/skills/installed",
        "/v1/shiju/skills/system?page=2&page_size=20&name=system+visual&tag_id=7",
        "/v1/shiju/skills/maintained?page=4&page_size=20&keyword=my+visual&tag_ids=2%2C1&company_tag_ids=5&statuses=draft%2Cpending_review%2Cpublished%2Cunpublished",
        "/v1/shiju/skills/31",
        "/v1/shiju/skills/31/readme",
        "/v1/shiju/skills/31/versions",
    ];
    for expected_path in expected_paths {
        let request = server.take_request();
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, expected_path);
        assert_eq!(request.header(SHIJU_CREDENTIAL_HEADER), Some(credential.expose()));
        assert!(!request.path.contains(credential.expose()));
    }
    server.join();
}

#[tokio::test]
async fn shiju_skill_client_omits_unselected_market_filters() {
    let page = json!({"items": [], "next_cursor": null, "has_more": false});
    let server = test_support::server(vec![(200, page.clone()), (200, page.clone()), (200, page)]);
    let credential = credential();

    for (tag_ids, company_tag_ids) in [(Some(2), None), (None, Some(5)), (None, None)] {
        server
            .client()
            .shiju_skill_search(
                &credential,
                &ShijuSkillSearchQuery {
                    query: None,
                    cursor: None,
                    page_size: 20,
                    tag_ids,
                    company_tag_ids,
                },
            )
            .await
            .expect("market search");
    }

    for expected_path in [
        "/v1/shiju/skills/search?page_size=20&tag_ids=2",
        "/v1/shiju/skills/search?page_size=20&company_tag_ids=5",
        "/v1/shiju/skills/search?page_size=20",
    ] {
        assert_eq!(server.take_request().path, expected_path);
    }
    server.join();
}

#[tokio::test]
async fn shiju_skill_client_uses_tag_catalog_paths() {
    let server = test_support::server(vec![
        (200, json!({"items": [{"id": 2, "name": "平台", "sort_order": 1}]})),
        (200, json!({"items": [{"id": 5, "name": "公司", "sort_order": 3}]})),
    ]);
    let credential = credential();

    let platform = server.client().shiju_skill_tags(&credential).await.unwrap();
    let company = server.client().shiju_tenant_skill_tags(&credential).await.unwrap();

    assert_eq!(platform.items[0].id, 2);
    assert_eq!(platform.items[0].name, "平台");
    assert_eq!(platform.items[0].sort_order, 1);
    assert_eq!(company.items[0].id, 5);
    assert_eq!(company.items[0].name, "公司");
    assert_eq!(company.items[0].sort_order, 3);
    for expected_path in ["/v1/shiju/skills/tags", "/v1/shiju/skills/tenant/tags"] {
        let request = server.take_request();
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, expected_path);
        assert_eq!(request.header(SHIJU_CREDENTIAL_HEADER), Some(credential.expose()));
        assert!(!request.path.contains(credential.expose()));
    }
    server.join();
}

#[test]
fn shiju_skill_contract_preserves_catalog_detail_and_version_shapes() {
    let catalog_fixture = json!({
        "items": [{
            "id": 30,
            "slug": "shiju-visual",
            "name": "识矩引擎视觉",
            "description": "品牌视觉规范",
            "level": "company",
            "modality": "local",
            "format": "zip",
            "company_id": "10011",
            "created_uid": "101027",
            "maintainer_uid": "101027",
            "status": "published",
            "is_all_company_accessible": 0,
            "current_version_id": 60,
            "created_at": "2026-07-28T09:55:24+08:00",
            "updated_at": "2026-07-28T09:56:15+08:00",
            "deleted_at": null,
            "is_installed": true,
            "tags": [{"id": 7, "name": "视觉", "sort_order": 1}]
        }],
        "next_cursor": "cursor-2",
        "has_more": true
    });
    let detail_fixture = json!({
        "id": 30,
        "slug": "shiju-visual",
        "name": "识矩引擎视觉",
        "description": "品牌视觉规范",
        "modality": "local",
        "format": "zip",
        "company_id": "10011",
        "created_uid": "101027",
        "maintainer_uid": "101027",
        "status": "published",
        "current_version_id": 60,
        "current_version": {
            "checksum": "sha256",
            "content_type": "zip",
            "download_expires_at": 1785246140,
            "download_url": "https://skill.shiju.cloud/content.zip?signature=detail-secret",
            "version": "0.1.1"
        },
        "created_at": "2026-07-28T09:55:24+08:00",
        "updated_at": "2026-07-28T09:56:15+08:00",
        "tags": [{"id": 7, "name": "视觉", "sort_order": 1}]
    });
    let versions_fixture = json!([{
        "id": 60,
        "skill_id": 30,
        "version": "0.1.1",
        "status": "published",
        "change_type": "patch",
        "changelog": "Initial publish",
        "checksum": "sha256",
        "content_type": "zip",
        "download_expires_at": 1785246140,
        "download_url": null,
        "created_at": "2026-07-28T09:55:25+08:00",
        "updated_at": "2026-07-28T09:56:15+08:00"
    }]);

    let catalog: ShijuSkillCursorPageDto =
        serde_json::from_value(catalog_fixture.clone()).expect("catalog contract");
    let detail: ShijuSkillDetailDto =
        serde_json::from_value(detail_fixture.clone()).expect("detail contract");
    let versions: Vec<ShijuSkillVersionDto> =
        serde_json::from_value(versions_fixture.clone()).expect("version contract");

    assert_eq!(catalog.items[0].tags[0].name, "视觉");
    assert_eq!(catalog.next_cursor.as_deref(), Some("cursor-2"));
    assert_eq!(
        detail.current_version.as_ref().and_then(|version| version.download_url.as_deref()),
        Some("https://skill.shiju.cloud/content.zip?signature=detail-secret")
    );
    assert!(detail.level.is_empty());
    assert_eq!(detail.is_all_company_accessible, 0);
    assert!(versions[0].download_url.is_none());
    assert_eq!(serde_json::to_value(catalog).unwrap(), catalog_fixture);
    assert_eq!(serde_json::to_value(versions).unwrap(), versions_fixture);

    let page: ShijuSkillPageDto<serde_json::Value> = serde_json::from_value(json!({
        "items": [],
        "page": 1,
        "page_size": 20,
        "total": 0
    }))
    .expect("page contract");
    assert_eq!(page.page_size, 20);
}

#[test]
fn shiju_skill_readme_contract_accepts_current_and_legacy_gateway_shapes() {
    let current: ShijuSkillReadmeDto = serde_json::from_value(json!({
        "markdown": "# 当前 README"
    }))
    .expect("current README contract");
    assert_eq!(current.markdown.as_deref(), Some("# 当前 README"));

    let legacy: ShijuSkillReadmeDto = serde_json::from_value(json!({
        "content": json!({
            "code": 200,
            "message": "",
            "data": {
                "skill_id": 21,
                "version": "3.0.0",
                "markdown": "# 识矩业务技能发布器"
            }
        })
        .to_string()
    }))
    .expect("legacy README contract");
    assert_eq!(legacy.markdown.as_deref(), Some("# 识矩业务技能发布器"));

    let legacy_plain_text: ShijuSkillReadmeDto = serde_json::from_value(json!({
        "content": "# 旧版纯文本 README"
    }))
    .expect("legacy plain-text README contract");
    assert_eq!(legacy_plain_text.markdown.as_deref(), Some("# 旧版纯文本 README"));

    let empty: ShijuSkillReadmeDto =
        serde_json::from_value(json!({"markdown": null})).expect("empty README contract");
    assert_eq!(empty.markdown, None);
}

#[test]
fn shiju_skill_contract_accepts_current_marketplace_and_tenant_shapes() {
    let marketplace: ShijuSkillCursorPageDto = serde_json::from_value(json!({
        "items": [{
            "id": 51,
            "slug": "shiju-app",
            "name": "识矩应用",
            "description": "创建与管理应用",
            "modality": "local",
            "format": "zip",
            "publisher_company_id": "10011",
            "publisher_kind": "platform",
            "marketplace_submission_id": 13,
            "marketplace_status": "approved",
            "marketplace_version": "1.2.3",
            "distribution_source": "marketplace",
            "content_type": "zip",
            "is_installed": true,
            "tags": [{"id": 2, "name": "识矩", "sort_order": 1}]
        }],
        "page": 1,
        "page_size": 20,
        "total": 1
    }))
    .expect("current marketplace contract");
    assert_eq!(marketplace.items[0].company_id, "10011");
    assert_eq!(marketplace.items[0].status, "approved");
    assert_eq!(marketplace.items[0].current_version_id, 0);
    assert!(!marketplace.has_more);

    let tenant: ShijuSkillPageDto<super::ShijuTenantSkillDto> = serde_json::from_value(json!({
        "items": [{
            "id": 42,
            "slug": "reimbursement-assistant-companion",
            "name": "报销助手",
            "description": "报销流程",
            "status": "published",
            "has_pending_review": false,
            "pending_review_version_id": null,
            "tags": [],
            "tenant_visibility": {
                "effective_mode": "tenant_open",
                "scope_row": {"department_ids": null, "user_ids": null}
            }
        }],
        "page": 1,
        "page_size": 20,
        "total": 1
    }))
    .expect("current tenant contract");
    assert_eq!(tenant.items[0].is_all_company_accessible, 0);
}

#[test]
fn shiju_marketplace_contract_tolerates_optional_display_metadata() {
    let marketplace: ShijuSkillCursorPageDto = serde_json::from_value(json!({
        "items": [{
            "id": 30,
            "slug": "shiju-visual",
            "name": "识矩引擎视觉",
            "description": "品牌视觉规范",
            "publisher_company_id": "10011",
            "marketplace_status": "",
            "is_installed": false,
            "modality": null,
            "format": null,
            "tags": null
        }],
        "next_cursor": null,
        "has_more": false
    }))
    .expect("optional marketplace metadata");

    let skill = &marketplace.items[0];
    assert!(skill.modality.is_empty());
    assert!(skill.format.is_empty());
    assert!(skill.tags.is_empty());
}

#[tokio::test]
async fn shiju_skill_mutations_do_not_serialize_credentials() {
    let server = test_support::server_raw(vec![
        (204, String::new()),
        (204, String::new()),
        (204, String::new()),
    ]);
    let credential = credential();

    server.client().shiju_skill_remote_install(&credential, 31).await.unwrap();
    server.client().shiju_skill_remote_install_version(&credential, 31, 88).await.unwrap();
    server.client().shiju_skill_remote_uninstall(&credential, 31).await.unwrap();

    let install = server.take_request();
    assert_eq!(install.method, "POST");
    assert_eq!(install.path, "/v1/shiju/skills/31/install");
    assert_eq!(install.body, serde_json::Value::Null);
    assert_eq!(install.header(SHIJU_CREDENTIAL_HEADER), Some(credential.expose()));

    let version_install = server.take_request();
    assert_eq!(version_install.method, "POST");
    assert_eq!(version_install.path, "/v1/shiju/skills/31/install");
    assert_eq!(version_install.body, json!({"version_id": 88}));
    assert_eq!(version_install.header(SHIJU_CREDENTIAL_HEADER), Some(credential.expose()));

    let uninstall = server.take_request();
    assert_eq!(uninstall.method, "DELETE");
    assert_eq!(uninstall.path, "/v1/shiju/skills/31/install");
    assert_eq!(uninstall.body, serde_json::Value::Null);
    assert_eq!(uninstall.header(SHIJU_CREDENTIAL_HEADER), Some(credential.expose()));

    for request in [install, version_install, uninstall] {
        assert!(!request.path.contains(credential.expose()));
        assert!(!request.body.to_string().contains(credential.expose()));
    }
    server.join();
}
