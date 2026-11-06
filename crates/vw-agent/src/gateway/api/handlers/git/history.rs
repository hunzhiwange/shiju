//! 网关内置 Git 库实现：提交对象读取，不访问客户端文件系统。
use vw_api_types::git::{
    GitHistoryCommitDto as HistoryCommit, GitHistoryDetailDto as HistoryDetail,
    GitHistoryFileDto as HistoryFile, GitRepositoryDto as RepositorySnapshot,
};

pub(super) fn snapshot(path: &str, limit: usize) -> Result<RepositorySnapshot, String> {
    snapshot_inner(path, limit).map_err(|error| error.to_string())
}

fn snapshot_inner(path: &str, limit: usize) -> Result<RepositorySnapshot, git2::Error> {
    let repo = git2::Repository::open(path)?;
    let remotes = repo.remotes()?;
    let names = remotes.iter().collect::<Result<Vec<_>, _>>()?;
    let remote = if names.contains(&Some("origin")) {
        Some("origin".to_owned())
    } else if remotes.len() == 1 {
        remotes.get(0)?.map(str::to_owned)
    } else {
        None
    };
    let mut result = RepositorySnapshot { remote, ..Default::default() };
    let head = match repo.head() {
        Ok(head) => head,
        Err(error)
            if matches!(
                error.code(),
                git2::ErrorCode::UnbornBranch | git2::ErrorCode::NotFound
            ) =>
        {
            return Ok(result);
        }
        Err(error) => return Err(error),
    };
    if head.is_branch() {
        result.branch = Some(head.shorthand()?.to_owned());
        let config = repo.config()?;
        if let Some(branch) = &result.branch {
            let remote = config.get_string(&format!("branch.{branch}.remote")).ok();
            let merge = config.get_string(&format!("branch.{branch}.merge")).ok();
            if let (Some(remote), Some(merge)) = (remote.as_ref(), merge.as_ref())
                && remote != "."
                && repo.find_remote(remote).is_ok()
                && let Some(target) = merge.strip_prefix("refs/heads/")
            {
                result.remote = Some(remote.clone());
                result.remote_branch = Some(target.to_owned());
                result.upstream = true;
                let local = repo.find_branch(branch, git2::BranchType::Local)?;
                if let Ok(upstream) = local.upstream()
                    && let (Some(local), Some(upstream)) = (head.target(), upstream.get().target())
                {
                    result.ahead_behind = Some(repo.graph_ahead_behind(local, upstream)?);
                }
            } else if remote.is_none() && merge.is_none() {
                result.remote_branch = result.branch.clone();
            } else {
                result.remote = None;
            }
        }
    }
    if limit == 0 {
        return Ok(result);
    }
    let mut walk = repo.revwalk()?;
    walk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
    walk.push_head()?;
    for oid in walk.take(limit + 1) {
        let commit = repo.find_commit(oid?)?;
        if result.commits.len() == limit {
            result.has_more = true;
            break;
        }
        result.commits.push(HistoryCommit {
            sha: commit.id().to_string(),
            summary: commit.summary()?.unwrap_or("（无标题）").into(),
            author: commit.author().name().unwrap_or("?").into(),
            time: commit.time().seconds(),
        });
    }
    Ok(result)
}

pub(super) fn detail(path: &str, sha: &str) -> Result<HistoryDetail, String> {
    let read = || -> Result<HistoryDetail, git2::Error> {
        let repo = git2::Repository::open(path)?;
        let commit = repo.find_commit(git2::Oid::from_str(sha)?)?;
        let tree = commit.tree()?;
        let parent = if commit.parent_count() > 0 { Some(commit.parent(0)?.tree()?) } else { None };
        let mut options = git2::DiffOptions::new();
        options.include_typechange(true);
        let mut diff = repo.diff_tree_to_tree(parent.as_ref(), Some(&tree), Some(&mut options))?;
        let mut find = git2::DiffFindOptions::new();
        find.renames(true);
        diff.find_similar(Some(&mut find))?;
        let mut files = Vec::new();
        let mut remaining_bytes = 4 * 1024 * 1024;
        for (index, delta) in diff.deltas().enumerate() {
            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default();
            let pair = read_blob_pair(&repo, &delta, &mut remaining_bytes)?;
            let (old_content, new_content, notice, insertions, deletions) = match pair {
                BlobPair::Text(old, new) => {
                    let (insertions, deletions) =
                        if let Some(patch) = git2::Patch::from_diff(&diff, index)? {
                            let (_, insertions, deletions) = patch.line_stats()?;
                            (insertions, deletions)
                        } else {
                            (0, 0)
                        };
                    (Some(old), Some(new), None, insertions, deletions)
                }
                BlobPair::Notice(notice) => (None, None, Some(notice), 0, 0),
            };
            let status = match delta.status() {
                git2::Delta::Added => vw_api_types::git::GitFileStatus::Added,
                git2::Delta::Deleted => vw_api_types::git::GitFileStatus::Deleted,
                git2::Delta::Modified => vw_api_types::git::GitFileStatus::Modified,
                git2::Delta::Renamed => vw_api_types::git::GitFileStatus::Renamed,
                git2::Delta::Copied => vw_api_types::git::GitFileStatus::Copied,
                git2::Delta::Typechange => vw_api_types::git::GitFileStatus::TypeChanged,
                _ => return Err(git2::Error::from_str("unsupported historical file status")),
            };
            files.push(HistoryFile {
                path,
                status,
                old_content,
                new_content,
                notice,
                insertions,
                deletions,
            });
        }
        Ok(HistoryDetail { message: commit.message().unwrap_or_default().into(), files })
    };
    read().map_err(|error| error.to_string())
}

enum BlobPair {
    Text(String, String),
    Notice(String),
}

// Git 对象来自选中提交及其第一父提交，不访问当前工作区或索引。
fn read_blob_pair(
    repo: &git2::Repository,
    delta: &git2::DiffDelta<'_>,
    budget: &mut usize,
) -> Result<BlobPair, git2::Error> {
    let mut contents = Vec::with_capacity(2);
    for file in [delta.old_file(), delta.new_file()] {
        if file.id().is_zero() {
            contents.push(String::new());
            continue;
        }
        if file.mode() == git2::FileMode::Commit {
            return Ok(BlobPair::Notice("子模块引用变化，请使用外部 Git 工具查看".into()));
        }
        let blob = repo.find_blob(file.id())?;
        if blob.is_binary() {
            return Ok(BlobPair::Notice("二进制文件，无法显示文本差异".into()));
        }
        if blob.size() > 512_000 || blob.size() > *budget {
            return Ok(BlobPair::Notice(
                "文件内容超过历史预览大小限制，请使用外部 Git 工具查看".into(),
            ));
        }
        *budget -= blob.size();
        contents.push(String::from_utf8_lossy(blob.content()).into_owned());
    }
    Ok(BlobPair::Text(contents.remove(0), contents.remove(0)))
}
