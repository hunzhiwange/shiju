//! 构建和读取项目文件索引。
//! 本模块用受限深度、数量上限和缓存校验控制文件系统扫描成本。

use super::*;
use std::fs;
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, UNIX_EPOCH};

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent dir should be created");
    }
    fs::write(path, content).expect("file should be written");
}

fn rel_files(root: &Path, files: Vec<String>) -> Vec<String> {
    files
        .into_iter()
        .map(|file| {
            Path::new(&file)
                .strip_prefix(root)
                .expect("file should be inside root")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect()
}

fn remove_cache_for(root: &str) {
    if let Some(path) = file_index_cache_path(root) {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn file_mtime_from_system_time_keeps_epoch_parts() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mtime = FileMtime::from_system_time(UNIX_EPOCH + Duration::new(7, 42))
            .expect("time after epoch should convert");

        assert_eq!(mtime, FileMtime { secs: 7, nanos: 42 });
        assert_eq!(FileMtime::from_system_time(UNIX_EPOCH - Duration::from_secs(1)), None);
    }
}

#[test]
fn dir_mtime_returns_none_for_missing_path() {
    let temp = tempfile::tempdir().expect("temp dir should be created");

    assert!(dir_mtime(&temp.path().join("missing")).is_none());
}

#[test]
fn hex_encode_encodes_utf8_bytes() {
    assert_eq!(hex_encode("Az0/中"), "417a302fe4b8ad");
}

#[test]
fn cache_map_and_cache_files_collect_subtree_in_declared_order() {
    let cache = FileIndexCache {
        version: FILE_INDEX_CACHE_VERSION,
        root: "root".to_string(),
        root_mtime: None,
        gitignore_mtime: None,
        entries: vec![
            DirEntry {
                path: "".to_string(),
                mtime: None,
                files: vec!["root/a.txt".to_string()],
                subdirs: vec!["nested".to_string(), "missing".to_string()],
            },
            DirEntry {
                path: "nested".to_string(),
                mtime: None,
                files: vec!["root/nested/b.txt".to_string()],
                subdirs: Vec::new(),
            },
        ],
    };

    let map = cache_map(&cache);
    assert_eq!(map.len(), 2);
    assert_eq!(cache_files(&cache), vec!["root/a.txt", "root/nested/b.txt"]);
}

#[test]
fn collect_cached_subtree_returns_when_output_is_already_at_limit() {
    let cache = FileIndexCache {
        version: FILE_INDEX_CACHE_VERSION,
        root: "root".to_string(),
        root_mtime: None,
        gitignore_mtime: None,
        entries: vec![DirEntry {
            path: "".to_string(),
            mtime: None,
            files: vec!["root/a.txt".to_string()],
            subdirs: vec!["nested".to_string()],
        }],
    };
    let map = cache_map(&cache);
    let mut files = vec!["filled".to_string(); FILE_INDEX_LIMIT];
    let mut entries = Vec::new();

    collect_cached_subtree("", &map, &mut files, &mut entries);

    assert_eq!(files.len(), FILE_INDEX_LIMIT);
    assert_eq!(entries.len(), 1);
}

#[test]
fn collect_cached_subtree_returns_before_subdirs_when_limit_is_reached_by_files() {
    let cache = FileIndexCache {
        version: FILE_INDEX_CACHE_VERSION,
        root: "root".to_string(),
        root_mtime: None,
        gitignore_mtime: None,
        entries: vec![DirEntry {
            path: "".to_string(),
            mtime: None,
            files: vec!["root/a.txt".to_string()],
            subdirs: vec!["nested".to_string()],
        }],
    };
    let map = cache_map(&cache);
    let mut files = vec!["filled".to_string(); FILE_INDEX_LIMIT - 1];
    let mut entries = Vec::new();

    collect_cached_subtree("", &map, &mut files, &mut entries);

    assert_eq!(files.len(), FILE_INDEX_LIMIT);
    assert_eq!(entries.len(), 1);
}

#[test]
fn collect_cached_subtree_leaves_outputs_unchanged_for_missing_entry() {
    let cache = FileIndexCache {
        version: FILE_INDEX_CACHE_VERSION,
        root: "root".to_string(),
        root_mtime: None,
        gitignore_mtime: None,
        entries: Vec::new(),
    };
    let map = cache_map(&cache);
    let mut files = vec!["existing.txt".to_string()];
    let mut entries = Vec::new();

    collect_cached_subtree("missing", &map, &mut files, &mut entries);

    assert_eq!(files, vec!["existing.txt"]);
    assert!(entries.is_empty());
}

#[test]
fn index_files_returns_sorted_files_and_skips_default_ignored_dirs() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let root = temp.path();
    write_file(&root.join("zeta.txt"), "z");
    write_file(&root.join("Alpha.txt"), "a");
    write_file(&root.join("src/lib.rs"), "lib");
    write_file(&root.join("target/debug.bin"), "ignored");
    write_file(&root.join("node_modules/pkg/index.js"), "ignored");

    let files = rel_files(root, index_files(root.to_string_lossy().to_string()));

    assert_eq!(files, vec!["Alpha.txt", "zeta.txt", "src/lib.rs"]);
}

#[test]
fn index_files_applies_gitignore_exact_glob_anchor_directory_and_negation_rules() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let root = temp.path();
    write_file(
        &root.join(".gitignore"),
        "\n# comments are ignored\n!\n!# nope\n/\n*.log\n!/keep.log\n/temp.txt\ncache/\nsrc/*.tmp\n[bad\n",
    );
    write_file(&root.join("keep.log"), "kept");
    write_file(&root.join("drop.log"), "ignored");
    write_file(&root.join("temp.txt"), "ignored");
    write_file(&root.join("nested/temp.txt"), "kept");
    write_file(&root.join("cache/file.txt"), "ignored");
    write_file(&root.join("cache.txt"), "kept");
    write_file(&root.join("src/drop.tmp"), "ignored");
    write_file(&root.join("src/keep.rs"), "kept");
    write_file(&root.join("[bad"), "ignored by exact fallback");

    let files = rel_files(root, index_files(root.to_string_lossy().to_string()));

    assert_eq!(
        files,
        vec![".gitignore", "cache.txt", "keep.log", "nested/temp.txt", "src/keep.rs"]
    );
}

#[test]
fn index_files_returns_empty_for_missing_or_file_root() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let file_root = temp.path().join("plain-file.txt");
    write_file(&file_root, "not a dir");

    assert!(index_files(file_root.to_string_lossy().to_string()).is_empty());
    assert!(index_files(temp.path().join("missing").to_string_lossy().to_string()).is_empty());
}

#[test]
fn refresh_file_index_saves_cache_and_load_file_index_reads_cached_files() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let root = temp.path();
    let root_string = root.to_string_lossy().to_string();
    remove_cache_for(&root_string);
    write_file(&root.join("src/main.rs"), "fn main() {}");
    write_file(&root.join("README.md"), "docs");

    let refreshed = rel_files(root, refresh_file_index(&root_string));
    let loaded = load_file_index(&root_string);
    let loaded_files = rel_files(root, loaded.files);

    assert_eq!(refreshed, vec!["README.md", "src/main.rs"]);
    assert_eq!(loaded_files, refreshed);
    assert!(loaded.needs_refresh);
    remove_cache_for(&root_string);
}

#[test]
fn load_file_index_requests_refresh_without_usable_cache() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let root = temp.path();
    let root_string = root.to_string_lossy().to_string();
    remove_cache_for(&root_string);

    let missing = load_file_index(&root_string);
    assert!(missing.files.is_empty());
    assert!(missing.needs_refresh);

    let wrong_version = FileIndexCache {
        version: FILE_INDEX_CACHE_VERSION.saturating_add(1),
        root: root_string.clone(),
        root_mtime: None,
        gitignore_mtime: gitignore_mtime(root),
        entries: Vec::new(),
    };
    save_file_index_cache(&root_string, &wrong_version);

    let invalid = load_file_index(&root_string);
    assert!(invalid.files.is_empty());
    assert!(invalid.needs_refresh);
    remove_cache_for(&root_string);
}

#[test]
fn load_file_index_cache_rejects_wrong_root_and_invalid_json() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let root = temp.path();
    let root_string = root.to_string_lossy().to_string();
    remove_cache_for(&root_string);

    let wrong_root = FileIndexCache {
        version: FILE_INDEX_CACHE_VERSION,
        root: "other-root".to_string(),
        root_mtime: None,
        gitignore_mtime: None,
        entries: Vec::new(),
    };
    save_file_index_cache(&root_string, &wrong_root);
    assert!(load_file_index_cache(&root_string).is_none());

    let path = file_index_cache_path(&root_string).expect("cache path should exist");
    fs::write(&path, "{not-json").expect("invalid cache should be written");
    assert!(load_file_index_cache(&root_string).is_none());
    remove_cache_for(&root_string);
}

#[test]
fn load_file_index_requests_refresh_when_gitignore_mtime_changes() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let root = temp.path();
    let root_string = root.to_string_lossy().to_string();
    remove_cache_for(&root_string);
    write_file(&root.join("visible.txt"), "visible");

    let stale_cache = FileIndexCache {
        version: FILE_INDEX_CACHE_VERSION,
        root: root_string.clone(),
        root_mtime: dir_mtime(root),
        gitignore_mtime: None,
        entries: vec![DirEntry {
            path: "".to_string(),
            mtime: dir_mtime(root),
            files: vec![root.join("visible.txt").to_string_lossy().to_string()],
            subdirs: Vec::new(),
        }],
    };
    save_file_index_cache(&root_string, &stale_cache);
    write_file(&root.join(".gitignore"), "*.tmp");

    let loaded = load_file_index(&root_string);

    assert!(loaded.files.is_empty());
    assert!(loaded.needs_refresh);
    remove_cache_for(&root_string);
}

#[test]
fn index_files_applies_name_only_globs_and_backslash_normalization() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let root = temp.path();
    write_file(&root.join(".gitignore"), "generated?.txt\nlogs\\\\\n");
    write_file(&root.join("generated1.txt"), "ignored");
    write_file(&root.join("generated10.txt"), "kept");
    write_file(&root.join("logs/app.txt"), "ignored");
    write_file(&root.join("src/generated2.txt"), "ignored by name glob");
    write_file(&root.join("src/keep.rs"), "kept");

    let files = rel_files(root, index_files(root.to_string_lossy().to_string()));

    assert_eq!(files, vec![".gitignore", "generated10.txt", "src/keep.rs"]);
}

#[test]
fn index_files_uses_last_matching_ignore_rule() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let root = temp.path();
    write_file(
        &root.join(".gitignore"),
        "*.tmp\n!important.tmp\nimportant.tmp\n*.log\n!keep.log\n",
    );
    write_file(&root.join("important.tmp"), "ignored again");
    write_file(&root.join("other.tmp"), "ignored");
    write_file(&root.join("drop.log"), "ignored");
    write_file(&root.join("keep.log"), "kept");
    write_file(&root.join("source.rs"), "kept");

    let files = rel_files(root, index_files(root.to_string_lossy().to_string()));

    assert_eq!(files, vec![".gitignore", "keep.log", "source.rs"]);
}

#[test]
fn index_files_matches_unanchored_path_rule_only_at_declared_relative_path() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let root = temp.path();
    write_file(&root.join(".gitignore"), "docs/private.txt\n");
    write_file(&root.join("private.txt"), "kept");
    write_file(&root.join("docs/private.txt"), "ignored");
    write_file(&root.join("nested/docs/private.txt"), "kept");

    let files = rel_files(root, index_files(root.to_string_lossy().to_string()));

    assert_eq!(files, vec![".gitignore", "private.txt", "nested/docs/private.txt"]);
}

#[test]
fn refresh_file_index_reuses_cached_unchanged_subtree_and_rescans_changed_root() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let root = temp.path();
    let root_string = root.to_string_lossy().to_string();
    remove_cache_for(&root_string);
    write_file(&root.join("stable/cached.txt"), "cached");
    write_file(&root.join("root.txt"), "root");

    let first = rel_files(root, refresh_file_index(&root_string));
    assert_eq!(first, vec!["root.txt", "stable/cached.txt"]);

    write_file(&root.join("new.txt"), "new");
    let second = rel_files(root, refresh_file_index(&root_string));

    assert_eq!(second, vec!["new.txt", "root.txt", "stable/cached.txt"]);
    remove_cache_for(&root_string);
}

#[test]
fn index_files_with_cache_reuses_unchanged_root_entry() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let root = temp.path();
    let root_string = root.to_string_lossy().to_string();
    write_file(&root.join("disk.txt"), "not scanned when cache matches");
    let cached_file = root.join("cached.txt").to_string_lossy().to_string();
    let root_mtime = dir_mtime(root);
    let cache = FileIndexCache {
        version: FILE_INDEX_CACHE_VERSION,
        root: root_string.clone(),
        root_mtime,
        gitignore_mtime: None,
        entries: vec![DirEntry {
            path: "".to_string(),
            mtime: root_mtime,
            files: vec![cached_file.clone()],
            subdirs: Vec::new(),
        }],
    };

    let updated = index_files_with_cache(&root_string, Some(&cache));

    assert_eq!(cache_files(&updated), vec![cached_file]);
}

#[test]
fn refresh_file_index_skips_default_ignored_directories() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let root = temp.path();
    let root_string = root.to_string_lossy().to_string();
    remove_cache_for(&root_string);
    write_file(&root.join("keep.txt"), "kept");
    write_file(&root.join("src/lib.rs"), "kept");
    write_file(&root.join(".git/config"), "ignored");
    write_file(&root.join("target/debug.bin"), "ignored");
    write_file(&root.join("node_modules/pkg/index.js"), "ignored");

    let files = rel_files(root, refresh_file_index(&root_string));

    assert_eq!(files, vec!["keep.txt", "src/lib.rs"]);
    remove_cache_for(&root_string);
}

#[test]
fn refresh_file_index_drops_cache_when_gitignore_changes() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let root = temp.path();
    let root_string = root.to_string_lossy().to_string();
    remove_cache_for(&root_string);
    write_file(&root.join("keep.txt"), "keep");
    write_file(&root.join("skip.tmp"), "skip");

    let first = rel_files(root, refresh_file_index(&root_string));
    assert_eq!(first, vec!["keep.txt", "skip.tmp"]);

    write_file(&root.join(".gitignore"), "*.tmp");
    let second = rel_files(root, refresh_file_index(&root_string));

    assert_eq!(second, vec![".gitignore", "keep.txt"]);
    remove_cache_for(&root_string);
}

#[test]
fn refresh_file_index_applies_anchored_exact_gitignore_rules_and_handles_file_root() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let root = temp.path();
    let root_string = root.to_string_lossy().to_string();
    remove_cache_for(&root_string);
    write_file(&root.join(".gitignore"), "!\n!# nope\n/\n/root-only.txt\n");
    write_file(&root.join("root-only.txt"), "ignored");
    write_file(&root.join("nested/root-only.txt"), "kept");

    let files = rel_files(root, refresh_file_index(&root_string));

    assert_eq!(files, vec![".gitignore", "nested/root-only.txt"]);
    assert!(refresh_file_index(&root.join("plain-file.txt").to_string_lossy()).is_empty());
    remove_cache_for(&root_string);
}

#[test]
fn refresh_file_index_treats_invalid_gitignore_glob_as_exact_name() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let root = temp.path();
    let root_string = root.to_string_lossy().to_string();
    remove_cache_for(&root_string);
    write_file(&root.join(".gitignore"), "[invalid\n");
    write_file(&root.join("[invalid"), "ignored by exact fallback");
    write_file(&root.join("[invalid.txt"), "kept");

    let files = rel_files(root, refresh_file_index(&root_string));

    assert_eq!(files, vec![".gitignore", "[invalid.txt"]);
    remove_cache_for(&root_string);
}

#[test]
fn refresh_file_index_applies_name_only_globs_and_backslash_normalization() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let root = temp.path();
    let root_string = root.to_string_lossy().to_string();
    remove_cache_for(&root_string);
    write_file(&root.join(".gitignore"), "generated?.txt\nlogs\\\\\n");
    write_file(&root.join("generated1.txt"), "ignored");
    write_file(&root.join("generated10.txt"), "kept");
    write_file(&root.join("logs/app.txt"), "ignored");
    write_file(&root.join("src/generated2.txt"), "ignored by name glob");
    write_file(&root.join("src/keep.rs"), "kept");

    let files = rel_files(root, refresh_file_index(&root_string));

    assert_eq!(files, vec![".gitignore", "generated10.txt", "src/keep.rs"]);
    remove_cache_for(&root_string);
}

#[test]
fn indexers_stop_before_files_beyond_max_depth() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let root = temp.path();
    let mut deep_dir = root.to_path_buf();
    for level in 0..=FILE_INDEX_MAX_DEPTH {
        deep_dir = deep_dir.join(format!("d{level}"));
    }
    write_file(&deep_dir.join("too-deep.txt"), "hidden");

    let indexed = index_files(root.to_string_lossy().to_string());
    let refreshed = refresh_file_index(&root.to_string_lossy());

    assert!(indexed.is_empty());
    assert!(refreshed.is_empty());
}

#[test]
fn indexers_include_files_at_max_depth() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let root = temp.path();
    let root_string = root.to_string_lossy().to_string();
    remove_cache_for(&root_string);
    let mut deepest_dir = root.to_path_buf();
    let mut relative_parts = Vec::new();
    for level in 0..FILE_INDEX_MAX_DEPTH {
        let part = format!("d{level}");
        deepest_dir = deepest_dir.join(&part);
        relative_parts.push(part);
    }
    write_file(&deepest_dir.join("boundary.txt"), "included");
    let expected = format!("{}/boundary.txt", relative_parts.join("/"));

    let indexed = rel_files(root, index_files(root_string.clone()));
    let refreshed = rel_files(root, refresh_file_index(&root_string));

    assert_eq!(indexed, vec![expected.clone()]);
    assert_eq!(refreshed, vec![expected]);
    remove_cache_for(&root_string);
}

#[cfg(target_os = "linux")]
#[test]
fn indexers_skip_non_utf8_file_paths() {
    use std::os::unix::ffi::OsStringExt;

    let temp = tempfile::tempdir().expect("temp dir should be created");
    let root = temp.path();
    let root_string = root.to_string_lossy().to_string();
    remove_cache_for(&root_string);
    let invalid_name = std::ffi::OsString::from_vec(vec![b'f', 0xff]);
    fs::write(root.join(invalid_name), "not representable as UTF-8")
        .expect("non-UTF-8 file should be written");

    assert!(index_files(root_string.clone()).is_empty());
    assert!(refresh_file_index(&root_string).is_empty());
    remove_cache_for(&root_string);
}
