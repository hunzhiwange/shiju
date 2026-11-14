use super::*;
use std::io::Cursor;
use zip::write::SimpleFileOptions;

enum Fixture<'a> {
    File(&'a str, &'a [u8]),
    Link(&'a str, &'a str),
    Directory(&'a str),
}

fn zip(entries: &[Fixture<'_>]) -> Cursor<Vec<u8>> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for entry in entries {
        match entry {
            Fixture::File(name, bytes) => {
                writer.start_file(*name, options).unwrap();
                writer.write_all(bytes).unwrap();
            }
            Fixture::Link(name, target) => writer.add_symlink(*name, *target, options).unwrap(),
            Fixture::Directory(name) => writer.add_directory(*name, options).unwrap(),
        }
    }
    writer.finish().unwrap()
}

#[test]
fn cp12_archive_success() {
    let entries = [
        Fixture::File("repo-main/skill/SKILL.md", b"# skill"),
        Fixture::File("repo-main/shared/data.txt", b"payload"),
        Fixture::Link("repo-main/skill/assets", "../shared"),
        Fixture::Link("repo-main/skill/copy.txt", "assets/data.txt"),
        Fixture::Link("repo-main/skill/chain.txt", "copy.txt"),
        Fixture::Directory("repo-main/empty/"),
    ];
    let tree = extract_repo_archive(zip(&entries)).unwrap();
    assert_eq!(fs::read(tree.path().join("skill/SKILL.md")).unwrap(), b"# skill");
    for path in ["skill/assets/data.txt", "skill/copy.txt", "skill/chain.txt"] {
        let path = tree.path().join(path);
        assert_eq!(fs::read(&path).unwrap(), b"payload");
        assert!(!fs::symlink_metadata(path).unwrap().file_type().is_symlink());
    }
    assert!(tree.path().join("empty").is_dir());
    let path = tree.path().to_path_buf();
    drop(tree);
    assert!(!path.exists());
}

#[test]
fn cp12_archive_local_retains_root_and_guard_owns_tree() {
    let base = tempfile::tempdir().unwrap();
    let input = base.path().join("skill.zip");
    fs::write(&input, zip(&[Fixture::File("skill/SKILL.md", b"local")]).into_inner()).unwrap();
    let tree = extract_local_zip(&input).unwrap();
    assert_eq!(fs::read(tree.path().join("skill/SKILL.md")).unwrap(), b"local");
    let path = tree.path().to_path_buf();
    drop(tree);
    assert!(!path.exists());
    assert!(input.exists());
}

#[test]
fn cp12_archive_rejects_without_side_effects() {
    let cases = vec![
        vec![Fixture::File("../escape", b"bad")],
        vec![Fixture::File("/escape", b"bad")],
        vec![Fixture::File("C:/escape", b"bad")],
        vec![Fixture::File("dir/../escape", b"bad")],
        vec![Fixture::File("dir\\escape", b"bad")],
        vec![Fixture::Link("link", "../escape")],
        vec![Fixture::Link("link", "/etc/passwd")],
        vec![Fixture::Link("link", "missing")],
        vec![Fixture::Link("link", "link")],
        vec![Fixture::Link("a", "b"), Fixture::Link("b", "a")],
        vec![Fixture::File("dir/file", b"ok"), Fixture::Link("dir/self", "..")],
        vec![Fixture::Link("dir", "elsewhere"), Fixture::File("dir/file", b"bad")],
        vec![Fixture::File("dir/file", b"ok"), Fixture::Link("dir", "elsewhere")],
        vec![Fixture::File("dir", b"ok"), Fixture::File("dir/file", b"bad")],
    ];
    let parent = tempfile::tempdir().unwrap();
    let staging = parent.path().join("staging");
    fs::create_dir(&staging).unwrap();
    fs::write(parent.path().join("escape"), b"untouched").unwrap();
    for (index, entries) in cases.iter().enumerate() {
        let result = extract(zip(entries), false, Some(&staging), &|| Ok(()));
        assert!(result.is_err(), "case {index}");
        assert_eq!(fs::read_dir(&staging).unwrap().count(), 0, "case {index}");
        assert_eq!(fs::read(parent.path().join("escape")).unwrap(), b"untouched");
    }
}

#[test]
fn cp12_archive_rejects_repository_root_escape_and_multiple_roots() {
    for entries in [
        vec![Fixture::File("repo/../escape", b"bad")],
        vec![Fixture::File("one/a", b"a"), Fixture::File("two/b", b"b")],
        vec![Fixture::File("root", b"not a directory")],
    ] {
        assert!(extract_repo_archive(zip(&entries)).is_err());
    }
}

#[test]
fn cp12_archive_enforces_entry_and_download_limits() {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..=MAX_ARCHIVE_ENTRIES {
        writer.add_directory(format!("d{index}/"), SimpleFileOptions::default()).unwrap();
    }
    assert!(extract_repo_archive(writer.finish().unwrap()).is_err());
    // A sparse file exercises the input-size guard without a 512 MiB allocation.
    let input = tempfile::tempfile().unwrap();
    input.set_len(MAX_ARCHIVE_DOWNLOAD_BYTES + 1).unwrap();
    assert!(extract(input, false, None, &|| Ok(())).is_err());
}

#[test]
fn cp12_archive_bounds_link_reads_and_charges_file_and_directory_expansion() {
    let mut total = 0;
    let mut target = Cursor::new(vec![b'x'; MAX_SYMLINK_TARGET_BYTES as usize + 100]);
    assert!(read_symlink_target(&mut target, &mut total).is_err());
    assert_eq!(target.position(), MAX_SYMLINK_TARGET_BYTES + 1);
    let parent = tempfile::tempdir().unwrap();
    let mut near_limit = MAX_ARCHIVE_TOTAL_BYTES - BLOCK_BYTES + 1;
    assert!(write_file(&mut Cursor::new(b""), &parent.path().join("empty"), &mut near_limit, &|| Ok(())).is_err());

    fs::write(parent.path().join("source"), b"data").unwrap();
    let entries = BTreeMap::from([
        (PathBuf::from("source"), Entry::File),
        (PathBuf::from("copy"), Entry::Link(PathBuf::from("source"))),
        (PathBuf::from("directory"), Entry::Directory),
        (PathBuf::from("dir-copy"), Entry::Link(PathBuf::from("directory"))),
    ]);
    for link in ["copy", "dir-copy"] {
        let mut near_limit = MAX_ARCHIVE_TOTAL_BYTES - BLOCK_BYTES + 1;
        assert!(materialize(Path::new(link), &entries, &BTreeMap::new(), parent.path(), &mut near_limit, &|| Ok(())).is_err());
    }
    let long = "x".repeat(MAX_SYMLINK_TARGET_BYTES as usize + 1);
    let staging = tempfile::tempdir().unwrap();
    assert!(extract(zip(&[Fixture::Link("link", &long)]), false, Some(staging.path()), &|| Ok(())).is_err());
    assert_eq!(fs::read_dir(staging.path()).unwrap().count(), 0);
}
