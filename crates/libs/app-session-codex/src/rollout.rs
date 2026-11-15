//! Read-only Codex rollout files, including native zstd compression.
//!
//! These helpers do not authorize paths; callers must resolve sources inside their
//! permitted execution-host roots before opening files.

use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

const COMPRESSED_SUFFIX: &str = ".zst";
const PLAIN_SUFFIX: &str = ".jsonl";
const READER_CAPACITY: usize = 1 << 20;

fn file_name(path: &Path) -> Option<&str> {
    path.file_name().and_then(|name| name.to_str())
}

/// 是否为压缩形态（`*.jsonl.zst`）。
pub fn is_compressed(path: &Path) -> bool {
    file_name(path).is_some_and(|name| {
        name.strip_suffix(COMPRESSED_SUFFIX)
            .is_some_and(|plain| plain.ends_with(PLAIN_SUFFIX))
    })
}

/// 是否为 rollout 文件（`*.jsonl` 或 `*.jsonl.zst`），只看扩展名。
pub fn is_rollout_file(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") || is_compressed(path)
}

/// 去掉 `.zst` 后的普通形态路径；普通路径原样返回。
pub fn logical_path(path: &Path) -> PathBuf {
    match file_name(path)
        .filter(|_| is_compressed(path))
        .and_then(|name| name.strip_suffix(COMPRESSED_SUFFIX))
    {
        Some(plain) => path.with_file_name(plain),
        None => path.to_path_buf(),
    }
}

/// 另一种形态的路径：`x.jsonl` ↔ `x.jsonl.zst`。不是 rollout 文件时返回 `None`。
pub fn sibling(path: &Path) -> Option<PathBuf> {
    if is_compressed(path) {
        return Some(logical_path(path));
    }
    if !is_rollout_file(path) {
        return None;
    }
    let name = file_name(path)?;
    Some(path.with_file_name(format!("{name}{COMPRESSED_SUFFIX}")))
}

/// 按行读取，压缩文件透明解压。
pub fn open_reader(path: &Path) -> io::Result<Box<dyn BufRead + Send>> {
    reader_from_file(path, File::open(path)?)
}

/// 同 [`open_reader`]，用调用方已打开的文件（调用方还要从同一句柄取元数据时用）。
pub fn reader_from_file(path: &Path, file: File) -> io::Result<Box<dyn BufRead + Send>> {
    if is_compressed(path) {
        let decoder = zstd::stream::read::Decoder::new(file)?;
        Ok(Box::new(BufReader::with_capacity(READER_CAPACITY, decoder)))
    } else {
        Ok(Box::new(BufReader::with_capacity(READER_CAPACITY, file)))
    }
}

/// 读出全文（UTF-8），压缩文件先解压。
pub fn read_to_string(path: &Path) -> io::Result<String> {
    if !is_compressed(path) {
        return std::fs::read_to_string(path);
    }
    let mut text = String::new();
    open_reader(path)?.read_to_string(&mut text)?;
    Ok(text)
}

/// 逻辑文件名（去掉 `.zst`）。`path` 可以是任意平台的路径字符串（游标里存的原样）。
pub fn logical_file_name(path: &str) -> &str {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    match name.strip_suffix(COMPRESSED_SUFFIX) {
        Some(plain) if plain.ends_with(PLAIN_SUFFIX) => plain,
        _ => name,
    }
}

/// 同一逻辑名的两种形态同时存在时（压缩或解压的瞬间）只留普通形态：
/// 它是 Codex 正在写的那份，内容不少于压缩那份。保持原有顺序。
pub fn dedupe_siblings(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let plain: HashSet<PathBuf> = paths
        .iter()
        .filter(|path| !is_compressed(path))
        .cloned()
        .collect();
    let mut seen = HashSet::new();
    paths
        .into_iter()
        .filter(|path| !is_compressed(path) || !plain.contains(&logical_path(path)))
        .filter(|path| seen.insert(logical_path(path)))
        .collect()
}
