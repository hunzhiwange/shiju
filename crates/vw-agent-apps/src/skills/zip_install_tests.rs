use std::{fs, io::Write, path::Path};
use super::{zip_install::ZipImport, local_install::LocalInstall};

fn archive(path: &Path, entries: &[(&str, &str)]) {
    let mut writer = zip::ZipWriter::new(fs::File::create(path).unwrap());
    for (name, content) in entries {
        writer.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
        writer.write_all(content.as_bytes()).unwrap();
    }
    writer.finish().unwrap();
}

#[test]
fn cp12_zip_install_success() {
    let fixture = tempfile::tempdir().unwrap();
    let path = fixture.path().join("skills.zip");
    archive(&path, &[("bundle/alpha/SKILL.md", "# Alpha"), ("bundle/beta/SKILL.md", "# Beta")]);
    let ssot = fixture.path().join("ssot");
    fs::create_dir(&ssot).unwrap();
    let import = ZipImport::prepare(&path).unwrap();
    assert_eq!(import.skills.len(), 2);
    for skill in &import.skills {
        let mut install = LocalInstall::prepare(&ssot, &skill.source, &skill.directory).unwrap();
        install.publish().unwrap();
        install.finish();
    }
    assert_eq!(fs::read_to_string(ssot.join("alpha/SKILL.md")).unwrap(), "# Alpha");
    assert!(LocalInstall::prepare(&ssot, &import.skills[0].source, "alpha").is_err());
    assert_eq!(fs::read_dir(&ssot).unwrap().count(), 2);
}

#[test]
fn cp12_zip_install_rejects_without_side_effects() {
    let fixture = tempfile::tempdir().unwrap();
    let path = fixture.path().join("skills.zip");
    archive(&path, &[("../escape/SKILL.md", "# Escape")]);
    assert!(ZipImport::prepare(&path).is_err());
    assert!(!fixture.path().join("escape").exists());
    archive(&path, &[("README.md", "no skill")]);
    assert!(ZipImport::prepare(&path).is_err());
    archive(&path, &[("SKILL.md", "# Root")]);
    let import = ZipImport::prepare(&path).unwrap();
    assert_eq!(import.skills[0].directory, "skills");
    assert!(import.skills[0].validate().is_ok());
}
