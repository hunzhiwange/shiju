use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ring::signature::{Ed25519KeyPair, KeyPair};
use sha2::{Digest, Sha256};
use vw_gateway_client::{SkillMarketCoordinateDto, SkillMarketSkillLocatorDto};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use super::install::{
    install_verified_market_artifact, install_verified_market_artifact_for_roots,
    install_verified_market_skill_set,
};
use super::provider::{
    SkillMarketArtifact, SkillMarketArtifactProof, SkillMarketSkillSetArtifact,
    SkillMarketSkillSetMemberArtifact, SkillMarketSkillSetVerificationPolicy,
};

const TEST_SEED: [u8; 32] = [7; 32];

#[derive(Clone)]
struct FixtureEntry {
    name: String,
    content: Vec<u8>,
    mode: u32,
    symlink: bool,
}

impl FixtureEntry {
    fn file(name: impl Into<String>, content: impl Into<Vec<u8>>) -> Self {
        Self { name: name.into(), content: content.into(), mode: 0o100644, symlink: false }
    }
}

fn coordinate() -> SkillMarketCoordinateDto {
    SkillMarketCoordinateDto {
        locator: SkillMarketSkillLocatorDto {
            source: "fixture".to_string(),
            namespace: "public".to_string(),
            slug: "safe-skill".to_string(),
        },
        version: "1.2.3".to_string(),
    }
}

fn archive(entries: &[FixtureEntry]) -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);
    for entry in entries {
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(entry.mode);
        if entry.symlink {
            writer
                .add_symlink(entry.name.as_str(), String::from_utf8_lossy(&entry.content), options)
                .expect("add fixture symlink");
        } else {
            writer.start_file(entry.name.as_str(), options).expect("start fixture file");
            writer.write_all(&entry.content).expect("write fixture file");
        }
    }
    writer.finish().expect("finish fixture archive").into_inner()
}

fn content_hash(entries: &[FixtureEntry]) -> String {
    let mut files = entries
        .iter()
        .filter(|entry| !ignored_hash_path(&entry.name))
        .map(|entry| {
            let file_hash = hex::encode(Sha256::digest(&entry.content));
            (entry.name.clone(), file_hash)
        })
        .collect::<Vec<_>>();
    files.sort_by(|left, right| left.0.cmp(&right.0));
    let manifest =
        files.into_iter().map(|(path, hash)| format!("{path}:{hash}\n")).collect::<String>();
    hex::encode(Sha256::digest(manifest.as_bytes()))
}

fn ignored_hash_path(path: &str) -> bool {
    path.split('/').any(|component| {
        component == "_meta.json"
            || component == "__MACOSX"
            || component == ".DS_Store"
            || component == "Thumbs.db"
            || component.starts_with("._")
    })
}

fn signed_artifact_for(
    coordinate: &SkillMarketCoordinateDto,
    entries: &[FixtureEntry],
) -> SkillMarketArtifact {
    let content_hash = content_hash(entries);
    let payload = format!(
        r#"{{"content_hash":"{content_hash}","skill_slug":"{}","skill_version":"{}","v":1}}"#,
        coordinate.locator.slug, coordinate.version
    );
    let key_pair = Ed25519KeyPair::from_seed_unchecked(&TEST_SEED).expect("fixture key");
    SkillMarketArtifact {
        archive: archive(entries),
        proof: SkillMarketArtifactProof {
            signed: true,
            payload: payload.clone(),
            signature_b64: STANDARD.encode(key_pair.sign(payload.as_bytes()).as_ref()),
            content_hash,
            key_id: "fixture-key-v1".to_string(),
            public_key_b64: STANDARD.encode(key_pair.public_key().as_ref()),
        },
    }
}

fn signed_artifact(entries: &[FixtureEntry]) -> SkillMarketArtifact {
    signed_artifact_for(&coordinate(), entries)
}

fn skills_root(project: &Path) -> PathBuf {
    project.join(".vibewindow").join("skills")
}

fn default_security() -> vw_config_types::skills::SkillSecurityConfig {
    vw_config_types::skills::SkillSecurityConfig::default()
}

fn assert_no_staging_residue(skills_root: &Path) {
    let entries = match fs::read_dir(skills_root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => panic!("read skills root: {error}"),
    };
    for entry in entries {
        let name =
            entry.expect("read skills root entry").file_name().to_string_lossy().into_owned();
        assert!(!name.starts_with(".skill-install-"), "staging residue: {name}");
    }
}

#[test]
fn market_skill_install_accepts_only_verified_atomic_artifact() {
    let project = tempfile::tempdir().expect("fixture project");
    let root = skills_root(project.path());
    let valid_entries = vec![
        FixtureEntry::file(
            "SKILL.md",
            "---\nname: safe-skill\ndescription: Safe fixture\n---\n# Safe\n",
        ),
        FixtureEntry::file("_meta.json", br#"{"untrusted":"discard me"}"#.to_vec()),
        FixtureEntry::file("references/guide.md", "# Guide\n"),
    ];

    let installed = install_verified_market_artifact(
        &root,
        &coordinate(),
        signed_artifact(&valid_entries),
        &default_security(),
    )
    .expect("verified artifact should install");

    assert_eq!(installed, root.join("fixture--public--safe-skill"));
    assert!(installed.join("SKILL.md").is_file());
    let metadata: serde_json::Value = serde_json::from_slice(
        &fs::read(installed.join("_meta.json")).expect("read safe metadata"),
    )
    .expect("parse safe metadata");
    assert_eq!(metadata["source"], "fixture");
    assert_eq!(metadata["namespace"], "public");
    assert_eq!(metadata["slug"], "safe-skill");
    assert_eq!(metadata["version"], "1.2.3");
    assert!(metadata.get("payload").is_none());
    assert!(metadata.get("signature").is_none());
    assert_no_staging_residue(&root);
}

#[test]
fn market_skill_install_applies_configured_static_audit() {
    let entries = vec![
        FixtureEntry::file(
            "SKILL.md",
            "---\nname: safe-skill\ndescription: Script fixture\n---\n# Script\n",
        ),
        FixtureEntry::file("run.sh", "#!/bin/sh\ntrue\n"),
    ];

    let default_project = tempfile::tempdir().expect("default-policy project");
    let default_root = skills_root(default_project.path());
    install_verified_market_artifact(
        &default_root,
        &coordinate(),
        signed_artifact(&entries),
        &default_security(),
    )
    .expect("default policy should allow static script finding");

    let rejecting_project = tempfile::tempdir().expect("rejecting-policy project");
    let rejecting_root = skills_root(rejecting_project.path());
    let reject_scripts = vw_config_types::skills::SkillSecurityConfig {
        reject_script_files: true,
        ..Default::default()
    };
    let error = install_verified_market_artifact(
        &rejecting_root,
        &coordinate(),
        signed_artifact(&entries),
        &reject_scripts,
    )
    .expect_err("enabled script check must reject market artifact");
    assert!(error.to_string().contains("Skill security audit failed"));
    assert!(!rejecting_root.join("fixture--public--safe-skill").exists());
    assert_no_staging_residue(&rejecting_root);
}

#[test]
fn market_skill_install_rejects_dangerous_artifacts_without_residue() {
    let valid_manifest = FixtureEntry::file(
        "SKILL.md",
        "---\nname: safe-skill\ndescription: Safe fixture\n---\n# Safe\n",
    );
    let cases = [
        ("tampered", {
            let mut artifact = signed_artifact(std::slice::from_ref(&valid_manifest));
            artifact.archive = archive(&[FixtureEntry::file(
                "SKILL.md",
                "---\nname: safe-skill\ndescription: Changed\n---\n# Changed\n",
            )]);
            artifact
        }),
        ("unsigned", {
            let mut artifact = signed_artifact(std::slice::from_ref(&valid_manifest));
            artifact.proof.signed = false;
            artifact
        }),
        (
            "path traversal",
            signed_artifact(&[
                valid_manifest.clone(),
                FixtureEntry::file("../escaped.txt", "escape"),
            ]),
        ),
        (
            "backslash escape",
            signed_artifact(&[
                valid_manifest.clone(),
                FixtureEntry::file(r"..\escaped.txt", "escape"),
            ]),
        ),
        (
            "symlink",
            signed_artifact(&[
                valid_manifest.clone(),
                FixtureEntry {
                    name: "current".to_string(),
                    content: b"/tmp/target".to_vec(),
                    mode: 0o120777,
                    symlink: true,
                },
            ]),
        ),
        ("too many files", {
            let mut entries = vec![valid_manifest.clone()];
            entries
                .extend((0..512).map(|index| FixtureEntry::file(format!("refs/{index}.txt"), "")));
            signed_artifact(&entries)
        }),
        (
            "expanded size",
            signed_artifact(&[
                valid_manifest.clone(),
                FixtureEntry::file("large.bin", vec![0_u8; 64 * 1024 * 1024 + 1]),
            ]),
        ),
        (
            "long path",
            signed_artifact(&[
                valid_manifest.clone(),
                FixtureEntry::file(format!("{}.txt", "a".repeat(241)), "long"),
            ]),
        ),
        ("missing SKILL.md", signed_artifact(&[FixtureEntry::file("README.md", "# Missing\n")])),
    ];

    for (name, artifact) in cases {
        let project = tempfile::tempdir().expect("fixture project");
        let root = skills_root(project.path());
        let error =
            install_verified_market_artifact(&root, &coordinate(), artifact, &default_security())
                .expect_err(name);
        assert!(!error.to_string().is_empty(), "{name}");
        assert!(!project.path().join("escaped.txt").exists(), "{name}");
        assert!(!root.join("fixture--public--safe-skill").exists(), "{name}");
        assert_no_staging_residue(&root);
    }
}

#[test]
fn market_skill_install_rejects_download_limit_device_entry_and_target_conflict() {
    let valid_manifest = FixtureEntry::file(
        "SKILL.md",
        "---\nname: safe-skill\ndescription: Safe fixture\n---\n# Safe\n",
    );

    let project = tempfile::tempdir().expect("fixture project");
    let root = skills_root(project.path());
    let mut oversized = signed_artifact(std::slice::from_ref(&valid_manifest));
    oversized.archive = vec![0_u8; 32 * 1024 * 1024 + 1];
    install_verified_market_artifact(&root, &coordinate(), oversized, &default_security())
        .expect_err("download limit must fail");
    assert_no_staging_residue(&root);

    let project = tempfile::tempdir().expect("fixture project");
    let root = skills_root(project.path());
    let mut device_archive =
        archive(&[valid_manifest.clone(), FixtureEntry::file("device", "not a regular file")]);
    mark_last_central_entry_as_device(&mut device_archive);
    let mut device = signed_artifact(&[
        valid_manifest.clone(),
        FixtureEntry::file("device", "not a regular file"),
    ]);
    device.archive = device_archive;
    install_verified_market_artifact(&root, &coordinate(), device, &default_security())
        .expect_err("device entry must fail");
    assert_no_staging_residue(&root);

    let project = tempfile::tempdir().expect("fixture project");
    let root = skills_root(project.path());
    let target = root.join("fixture--public--safe-skill");
    fs::create_dir_all(&target).expect("create conflicting target");
    fs::write(target.join("marker"), "keep").expect("write conflict marker");
    install_verified_market_artifact(
        &root,
        &coordinate(),
        signed_artifact(std::slice::from_ref(&valid_manifest)),
        &default_security(),
    )
    .expect_err("target conflict must fail");
    assert_eq!(fs::read_to_string(target.join("marker")).expect("read conflict marker"), "keep");
    assert_no_staging_residue(&root);
}

#[test]
fn market_skill_multi_target_install_rolls_back_completed_targets() {
    let temp = tempfile::tempdir().expect("fixture roots");
    let first_root = temp.path().join("first/skills");
    let second_root = temp.path().join("second/skills");
    let target_name = "fixture--public--safe-skill";
    let conflicting = second_root.join(target_name);
    fs::create_dir_all(&conflicting).expect("create conflicting target");
    fs::write(conflicting.join("marker"), "keep").expect("write conflict marker");
    let entries = [FixtureEntry::file(
        "SKILL.md",
        "---\nname: safe-skill\ndescription: Safe fixture\n---\n# Safe\n",
    )];

    let error = install_verified_market_artifact_for_roots(
        &[first_root.clone(), second_root.clone()],
        &coordinate(),
        signed_artifact(&entries),
        &default_security(),
    )
    .expect_err("second target conflict must roll back first target");

    assert!(!error.to_string().is_empty());
    assert!(!first_root.join(target_name).exists());
    assert_eq!(
        fs::read_to_string(conflicting.join("marker")).expect("read conflict marker"),
        "keep"
    );
}

#[test]
fn market_skillset_install_is_atomic_and_records_source_membership() {
    let project = tempfile::tempdir().expect("fixture project");
    let root = skills_root(project.path());
    let member_coordinates = [
        SkillMarketCoordinateDto {
            locator: SkillMarketSkillLocatorDto {
                source: "fixture".to_string(),
                namespace: "public".to_string(),
                slug: "first-skill".to_string(),
            },
            version: "1.0.0".to_string(),
        },
        SkillMarketCoordinateDto {
            locator: SkillMarketSkillLocatorDto {
                source: "fixture".to_string(),
                namespace: "tools".to_string(),
                slug: "second-skill".to_string(),
            },
            version: "2.0.0".to_string(),
        },
    ];
    let member_entries = [
        vec![
            FixtureEntry::file(
                "SKILL.md",
                "---\nname: first-skill\ndescription: First\n---\n# First\n",
            ),
            FixtureEntry::file("references/nested/guide.md", "# Guide\n"),
        ],
        vec![FixtureEntry::file(
            "SKILL.md",
            "---\nname: second-skill\ndescription: Second\n---\n# Second\n",
        )],
    ];
    let suite = fixture_skill_set_artifact(&member_coordinates, &member_entries);

    let installed = install_verified_market_skill_set(&root, suite, &default_security())
        .expect("valid suite should install atomically");

    assert_eq!(installed.len(), 3);
    let meta = root.join("fixture--skillset--fixture-suite");
    assert!(meta.join("SKILL.md").is_file());
    assert!(root.join("fixture--public--first-skill/references/nested/guide.md").is_file());
    let meta_coordinate = SkillMarketCoordinateDto {
        locator: SkillMarketSkillLocatorDto {
            source: "fixture".to_string(),
            namespace: "skillset".to_string(),
            slug: "fixture-suite".to_string(),
        },
        version: "3.0.0".to_string(),
    };
    let expected_packages = member_coordinates
        .iter()
        .map(|coordinate| (coordinate, "skill"))
        .chain(std::iter::once((&meta_coordinate, "meta-skill")));
    for (coordinate, package_type) in expected_packages {
        let path = root.join(format!(
            "{}--{}--{}",
            coordinate.locator.source, coordinate.locator.namespace, coordinate.locator.slug
        ));
        let metadata: serde_json::Value = serde_json::from_slice(
            &fs::read(path.join("_meta.json")).expect("read suite metadata"),
        )
        .expect("parse suite metadata");
        assert_eq!(metadata["source"], "fixture");
        assert_eq!(metadata["package_type"], package_type);
        assert_eq!(metadata["source_pack"], "fixture-suite");
        assert_eq!(metadata["namespace"], coordinate.locator.namespace);
        assert_eq!(metadata["slug"], coordinate.locator.slug);
        assert_eq!(metadata["version"], coordinate.version);
    }
    assert_no_staging_residue(&root);

    let conflict_project = tempfile::tempdir().expect("conflict project");
    let conflict_root = skills_root(conflict_project.path());
    let existing = conflict_root.join("fixture--tools--second-skill");
    fs::create_dir_all(&existing).expect("create member conflict");
    fs::write(existing.join("marker"), "keep").expect("write conflict marker");
    install_verified_market_skill_set(
        &conflict_root,
        fixture_skill_set_artifact(&member_coordinates, &member_entries),
        &default_security(),
    )
    .expect_err("member conflict must reject the entire transaction");
    assert!(!conflict_root.join("fixture--public--first-skill").exists());
    assert!(!conflict_root.join("fixture--skillset--fixture-suite").exists());
    assert_eq!(fs::read_to_string(existing.join("marker")).expect("read marker"), "keep");
    assert_no_staging_residue(&conflict_root);

    let duplicate_project = tempfile::tempdir().expect("duplicate project");
    let duplicate_root = skills_root(duplicate_project.path());
    let mut duplicate = fixture_skill_set_artifact(&member_coordinates, &member_entries);
    duplicate.members.push(duplicate.members[0].clone());
    install_verified_market_skill_set(&duplicate_root, duplicate, &default_security())
        .expect_err("duplicate member coordinate must fail");
    assert_no_staging_residue(&duplicate_root);

    let unsafe_project = tempfile::tempdir().expect("unsafe project");
    let unsafe_root = skills_root(unsafe_project.path());
    let mut unsafe_suite = fixture_skill_set_artifact(&member_coordinates, &member_entries);
    unsafe_suite.archive = archive(&[FixtureEntry::file("../escaped", "escape")]);
    install_verified_market_skill_set(&unsafe_root, unsafe_suite, &default_security())
        .expect_err("unsafe suite archive must fail");
    assert!(!unsafe_project.path().join("escaped").exists());
    assert_no_staging_residue(&unsafe_root);

    let policy_project = tempfile::tempdir().expect("policy project");
    let policy_root = skills_root(policy_project.path());
    let mut unsupported = fixture_skill_set_artifact(&member_coordinates, &member_entries);
    unsupported.verification =
        SkillMarketSkillSetVerificationPolicy::Unsupported("unknown".to_string());
    install_verified_market_skill_set(&policy_root, unsupported, &default_security())
        .expect_err("unregistered verification policy must fail");
    assert_no_staging_residue(&policy_root);
}

fn fixture_skill_set_artifact(
    coordinates: &[SkillMarketCoordinateDto],
    member_entries: &[Vec<FixtureEntry>],
) -> SkillMarketSkillSetArtifact {
    let identify = "---\nname: fixture-suite\ndescription: Fixture suite\nversion: 3.0.0\npackage_type: meta-skill\n---\n# Fixture Suite\n";
    let identify_hash = hex::encode(Sha256::digest(identify.as_bytes()));
    let manifest = serde_json::json!({
        "manifestVersion": "1",
        "type": "skillset",
        "slug": "fixture-suite",
        "skillSlugs": coordinates
            .iter()
            .map(|coordinate| coordinate.locator.slug.as_str())
            .collect::<Vec<_>>(),
        "skills": coordinates
            .iter()
            .map(|coordinate| serde_json::json!({"slug": coordinate.locator.slug}))
            .collect::<Vec<_>>(),
        "files": [{
            "path": "identify.md",
            "sha256": identify_hash,
            "size": identify.len()
        }]
    });
    SkillMarketSkillSetArtifact {
        source: "fixture".to_string(),
        slug: "fixture-suite".to_string(),
        verification: SkillMarketSkillSetVerificationPolicy::TransportAndAudit,
        archive: archive(&[
            FixtureEntry::file(
                "manifest.json",
                serde_json::to_vec(&manifest).expect("serialize suite manifest"),
            ),
            FixtureEntry::file("identify.md", identify),
        ]),
        members: coordinates
            .iter()
            .zip(member_entries)
            .map(|(coordinate, entries)| SkillMarketSkillSetMemberArtifact {
                coordinate: coordinate.clone(),
                artifact: signed_artifact_for(coordinate, entries),
            })
            .collect(),
    }
}

fn mark_last_central_entry_as_device(archive: &mut [u8]) {
    let signature = [0x50, 0x4b, 0x01, 0x02];
    let offset = archive
        .windows(signature.len())
        .rposition(|window| window == signature)
        .expect("central directory entry");
    let external_attributes = (0o020666_u32) << 16;
    archive[(offset + 38)..(offset + 42)].copy_from_slice(&external_attributes.to_le_bytes());
}
