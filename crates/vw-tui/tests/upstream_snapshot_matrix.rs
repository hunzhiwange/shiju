//! 审计复制的上游快照、v3 新快照与允许差异文档是否保持完整矩阵。
//! 若本地上游源码可用，还会逐文件比较内容；缺失时仍执行仓库内自洽检查。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const UPSTREAM_TUI: &str = "/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui";
const UPSTREAM_SNAPSHOT_COUNT: usize = 689;
const RELOCATED_UPSTREAM_SNAPSHOT_COUNT: usize = 20;
const V3_SNAPSHOT_COUNT: usize = 50;

#[test]
// 数量、命名、残留 `.new` 文件和白名单分类共同防止快照静默漂移。
fn upstream_snapshot_matrix() {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let snapshots = snapshot_files(&crate_root);
    let upstream = named_snapshots(&snapshots, "codex_tui__");
    let v3 = named_snapshots(&snapshots, "vw_tui__");
    let relocated = snapshot_files(&crate_root.join("upstream_snapshots"));

    assert_eq!(upstream.len(), UPSTREAM_SNAPSHOT_COUNT);
    assert_eq!(v3.len(), V3_SNAPSHOT_COUNT);
    assert_eq!(snapshots.len(), UPSTREAM_SNAPSHOT_COUNT + V3_SNAPSHOT_COUNT);
    assert_eq!(relocated.len(), RELOCATED_UPSTREAM_SNAPSHOT_COUNT);
    assert!(snapshots.iter().all(|path| path.extension().is_some_and(|ext| ext == "snap")));
    assert!(snapshot_files_with_extension(&crate_root, "new").is_empty());

    let source_root = Path::new(UPSTREAM_TUI);
    if source_root.is_dir() {
        let source = named_snapshots(&snapshot_files(source_root), "codex_tui__");
        assert_eq!(source.len(), UPSTREAM_SNAPSHOT_COUNT);
        assert_eq!(upstream.keys().collect::<Vec<_>>(), source.keys().collect::<Vec<_>>());
        for (name, source_path) in source {
            let target_path = upstream.get(&name).expect("upstream snapshot must be retained");
            assert_eq!(
                fs::read(target_path).expect("retained snapshot must be readable"),
                fs::read(source_path).expect("source snapshot must be readable"),
                "retained upstream snapshot changed: {name}"
            );
        }
    }

    let whitelist = fs::read_to_string(crate_root.join("UPSTREAM.md"))
        .expect("UPSTREAM.md must document allowed product-copy differences");
    for category in ["品牌", "命令", "URL", "会话数据", "状态文案"] {
        assert!(whitelist.contains(category), "missing allowed-difference category: {category}");
    }

    let lib = fs::read_to_string(crate_root.join("src/lib.rs"))
        .expect("v3 compilation root must be readable");
    assert!(lib.contains("pub mod chatwidget;"));
    assert!(lib.contains("pub use chatwidget::{ChatWidget, ChatWidgetConfig, V3ChatStatus};"));
}

fn named_snapshots(paths: &[PathBuf], prefix: &str) -> BTreeMap<String, PathBuf> {
    // 按文件名前缀建立有序映射，便于比较上游与复制目录的同名集合。
    paths
        .iter()
        .filter_map(|path| {
            let name = path.file_name()?.to_str()?;
            name.starts_with(prefix).then(|| (name.to_string(), path.clone()))
        })
        .collect()
}

fn snapshot_files(root: &Path) -> Vec<PathBuf> {
    snapshot_files_with_extension(root, "snap")
}

fn snapshot_files_with_extension(root: &Path, extension: &str) -> Vec<PathBuf> {
    // 递归收集后排序，消除目录遍历顺序对断言的影响。
    let mut files = Vec::new();
    collect_files(root, extension, &mut files);
    files.sort();
    files
}

fn collect_files(root: &Path, extension: &str, files: &mut Vec<PathBuf>) {
    // 不可读目录按空集合处理，使可选上游检出路径保持非阻塞。
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, extension, files);
        } else if path.extension().is_some_and(|ext| ext == extension) {
            files.push(path);
        }
    }
}
