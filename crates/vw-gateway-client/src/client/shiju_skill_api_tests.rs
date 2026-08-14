use serde_json::json;

use super::{
    SHIJU_CREDENTIAL_HEADER, ShijuCompanySkillQuery, ShijuCredential, ShijuMaintainedSkillQuery,
    ShijuSkillCursorPageDto, ShijuSkillDetailDto, ShijuSkillPageDto, ShijuSkillSearchQuery,
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
        (200, json!({"items": [], "page": 3, "page_size": 20, "total": 0})),
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
        (200, json!([])),
    ]);
    let credential = credential();

    let search = ShijuSkillSearchQuery {
        query: Some("visual guide".to_string()),
        cursor: Some("cursor/next".to_string()),
        page_size: 20,
    };
    let system = ShijuSystemSkillQuery {
        page: 2,
        page_size: 20,
        name: Some("system visual".to_string()),
        tag_id: Some(7),
    };
    let company = ShijuCompanySkillQuery {
        page: 3,
        page_size: 20,
        keyword: Some("company visual".to_string()),
        tag_id: Some(8),
        status: Some("published".to_string()),
    };
    let maintained = ShijuMaintainedSkillQuery {
        page: 4,
        page_size: 20,
        keyword: Some("my visual".to_string()),
        tag_id: Some(9),
    };

    server.client().shiju_skill_search(&credential, &search).await.unwrap();
    server.client().shiju_skill_installed(&credential).await.unwrap();
    server.client().shiju_system_skills(&credential, &system).await.unwrap();
    server.client().shiju_company_skills(&credential, &company).await.unwrap();
    server.client().shiju_maintained_skills(&credential, &maintained).await.unwrap();
    server.client().shiju_skill_detail(&credential, 31).await.unwrap();
    server.client().shiju_skill_versions(&credential, 31).await.unwrap();

    let expected_paths = [
        "/v1/shiju/skills/search?page_size=20&q=visual+guide&cursor=cursor%2Fnext",
        "/v1/shiju/skills/installed",
        "/v1/shiju/skills/system?page=2&page_size=20&name=system+visual&tag_id=7",
        "/v1/shiju/skills/company?page=3&page_size=20&keyword=company+visual&tag_id=8&status=published",
        "/v1/shiju/skills/maintained?page=4&page_size=20&keyword=my+visual&tag_id=9",
        "/v1/shiju/skills/31",
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
        "level": "company",
        "modality": "local",
        "format": "zip",
        "company_id": "10011",
        "created_uid": "101027",
        "maintainer_uid": "101027",
        "status": "published",
        "is_all_company_accessible": 0,
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
    assert!(versions[0].download_url.is_none());
    assert_eq!(serde_json::to_value(catalog).unwrap(), catalog_fixture);
    assert_eq!(serde_json::to_value(detail).unwrap(), detail_fixture);
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
