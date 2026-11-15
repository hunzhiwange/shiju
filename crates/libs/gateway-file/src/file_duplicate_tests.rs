use super::duplicate_entry;
use std::fs;
use std::path::Path;

#[test]
fn duplicate_file_preserves_extension_and_existing_entries() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    fs::create_dir(root.join("nested")).unwrap();
    fs::write(root.join("nested/脚本.sh"), "source").unwrap();
    fs::write(root.join("nested/脚本 copy.sh"), "existing").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(root.join("nested/脚本.sh"), fs::Permissions::from_mode(0o750))
            .unwrap();
    }

    std::thread::scope(|scope| {
        let first = scope.spawn(|| duplicate_entry(&root, Path::new("nested/脚本.sh")));
        let second = scope.spawn(|| duplicate_entry(&root, Path::new("nested/脚本.sh")));
        first.join().unwrap().unwrap();
        second.join().unwrap().unwrap();
    });

    assert_eq!(fs::read_to_string(root.join("nested/脚本.sh")).unwrap(), "source");
    assert_eq!(fs::read_to_string(root.join("nested/脚本 copy.sh")).unwrap(), "existing");
    for name in ["脚本 copy 2.sh", "脚本 copy 3.sh"] {
        let copy = root.join("nested").join(name);
        assert_eq!(fs::read_to_string(&copy).unwrap(), "source");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(copy).unwrap().permissions().mode() & 0o777, 0o750);
        }
    }
}

#[test]
fn duplicate_directory_preserves_nested_files_and_empty_directories() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    fs::create_dir_all(root.join("dev.v1/nested/empty")).unwrap();
    fs::write(root.join("dev.v1/nested/config"), "source").unwrap();
    fs::create_dir(root.join("dev.v1 copy")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(root.join("dev.v1"), fs::Permissions::from_mode(0o700)).unwrap();
    }

    duplicate_entry(&root, Path::new("dev.v1")).unwrap();

    assert!(root.join("dev.v1 copy 2/nested/empty").is_dir());
    assert_eq!(fs::read_to_string(root.join("dev.v1 copy 2/nested/config")).unwrap(), "source");
    assert!(fs::read_dir(root.join("dev.v1 copy")).unwrap().next().is_none());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(root.join("dev.v1 copy 2")).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700);
    }
}

#[cfg(unix)]
#[test]
fn duplicate_directory_discards_staged_copy_when_a_link_is_encountered() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    fs::create_dir(root.join("source")).unwrap();
    fs::write(root.join("source/file"), "source").unwrap();
    std::os::unix::fs::symlink(temp.path(), root.join("source/link")).unwrap();

    assert!(duplicate_entry(&root, Path::new("source")).is_err());
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
}
