//! 技能中心模型测试，验证不同端点的数据形状与下载地址信任规则。
//! 不可信或携带凭据的地址应被剔除，同时保留其余版本元数据供界面展示。

use super::models::ShijuPackageId;
use super::*;

#[test]
fn catalog_page_models_preserve_endpoint_specific_shapes() {
    fn accepts_cursor_page(_: CursorPage<MarketplaceSkill>) {}
    fn accepts_page(_: Page<SystemSkill>) {}
    fn accepts_tenant_page(_: Page<TenantSkill>) {}
    fn accepts_installed(_: Vec<InstalledSkill>) {}
    fn accepts_tag(_: SkillTag) {}

    let _ =
        (accepts_cursor_page, accepts_page, accepts_tenant_page, accepts_installed, accepts_tag);
}

#[test]
fn skill_download_url_rejects_untrusted_or_credentialed_urls() {
    let signed =
        SkillDownloadUrl::parse("https://skill.shiju.cloud/files/skill.zip?signature=secret-value")
            .expect("trusted signed URL");

    assert_eq!(signed.as_str(), "https://skill.shiju.cloud/files/skill.zip?signature=secret-value");
    assert_eq!(format!("{signed:?}"), "SkillDownloadUrl(<redacted>)");

    for untrusted in [
        "http://skill.shiju.cloud/files/skill.zip?signature=secret-value",
        "https://evil.example/files/skill.zip?signature=secret-value",
        "https://skill.shiju.cloud.evil.example/files/skill.zip?signature=secret-value",
        "https://user:password@skill.shiju.cloud/files/skill.zip?signature=secret-value",
        "https://skill.shiju.cloud:444/files/skill.zip?signature=secret-value",
    ] {
        let error = SkillDownloadUrl::parse(untrusted).expect_err("reject untrusted URL");
        assert!(!format!("{error:?}").contains("secret-value"));
    }
}

#[test]
fn skill_version_keeps_metadata_when_download_url_is_unavailable() {
    let version = serde_json::from_value::<SkillVersion>(serde_json::json!({
        "id": 60,
        "skill_id": 30,
        "version": "0.1.1",
        "status": "published",
        "change_type": "patch",
        "download_url": "https://untrusted.example/content.zip"
    }))
    .expect("version metadata remains readable");

    assert_eq!(version.id, 60);
    assert_eq!(version.version, "0.1.1");
    assert!(version.download_url.is_none());
}

#[test]
fn shiju_gateway_dtos_preserve_download_url_validation() {
    let detail_dto: vw_gateway_client::ShijuSkillDetailDto =
        serde_json::from_value(serde_json::json!({
            "id": 30,
            "slug": "shiju-visual",
            "name": "识矩视觉",
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
                "download_expires_at": 1_800_000_000,
                "download_url":
                    "https://skill.shiju.cloud/content.zip?signature=detail-secret",
                "version": "0.1.1"
            },
            "created_at": "2026-07-28T09:55:24+08:00",
            "updated_at": "2026-07-28T09:56:15+08:00",
            "tags": []
        }))
        .expect("detail dto");
    let detail = SkillDetail::try_from(detail_dto.clone()).expect("trusted detail");
    assert_eq!(
        detail.current_version.as_ref().map(|version| version.download_url.as_str()),
        Some("https://skill.shiju.cloud/content.zip?signature=detail-secret")
    );
    assert!(!format!("{detail:?}").contains("detail-secret"));

    let mut untrusted = detail_dto;
    untrusted.current_version.as_mut().unwrap().download_url =
        Some("https://untrusted.example/content.zip?signature=leak-secret".to_string());
    let error = SkillDetail::try_from(untrusted).expect_err("untrusted detail URL");
    assert!(!error.contains("leak-secret"));

    let version_dto: vw_gateway_client::ShijuSkillVersionDto =
        serde_json::from_value(serde_json::json!({
            "id": 60,
            "skill_id": 30,
            "version": "0.1.1",
            "download_url": "https://untrusted.example/content.zip?signature=version-secret"
        }))
        .expect("version dto");
    let version = SkillVersion::from(version_dto);
    assert!(version.download_url.is_none());
    assert!(!format!("{version:?}").contains("version-secret"));
}

#[test]
fn shiju_package_ids_match_supported_targets() {
    let packages = ShijuPackageId::ALL;
    assert_eq!(packages.len(), 4);
    assert_eq!(packages[0].as_str(), "shiju-darwin-amd64");
    assert_eq!(packages[0].name(), "macOS Intel");
    assert_eq!(packages[0].description(), "适用于 Intel 芯片 Mac。");
    assert_eq!(packages[1].as_str(), "shiju-darwin-arm64");
    assert_eq!(packages[2].as_str(), "shiju-linux-amd64");
    assert_eq!(packages[3].as_str(), "shiju-windows-amd64");

    let current = ShijuPackageId::current();
    if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        assert_eq!(current, Some(ShijuPackageId::DarwinAmd64));
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        assert_eq!(current, Some(ShijuPackageId::DarwinArm64));
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        assert_eq!(current, Some(ShijuPackageId::LinuxAmd64));
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        assert_eq!(current, Some(ShijuPackageId::WindowsAmd64));
    } else {
        assert_eq!(current, None);
    }
}
