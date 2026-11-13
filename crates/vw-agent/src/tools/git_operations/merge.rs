//! Explicit merge with Git-owned target reference locking and in-memory conflict detection.

use super::*;

impl GitOperationsTool {
    pub(super) async fn git_merge(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let result = (|| -> anyhow::Result<serde_json::Value> {
            let source = args
                .get("source")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing 'source' parameter"))?;
            let target = args
                .get("target")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing 'target' parameter"))?;
            Self::validate_branch_name(source)?;
            Self::validate_branch_name(target)?;
            if source == target {
                anyhow::bail!("Source and target must differ");
            }
            let source_ref = format!("refs/heads/{source}");
            let target_ref = format!("refs/heads/{target}");
            if !git2::Reference::is_valid_name(&source_ref)
                || !git2::Reference::is_valid_name(&target_ref)
            {
                anyhow::bail!("Invalid local branch name");
            }
            let repo = self.open_repository()?;
            // Git owns this cross-process lock; dropping the transaction releases it on every error.
            let mut transaction = repo.transaction()?;
            transaction.lock_ref(&target_ref)?;
            let head = repo.head()?;
            if head.name()? != target_ref {
                anyhow::bail!("Merge target must be checked out in the active worktree");
            }
            if repo.state() != git2::RepositoryState::Clean {
                anyhow::bail!("Repository has an operation in progress");
            }
            let mut options = StatusOptions::new();
            options.include_untracked(true).recurse_untracked_dirs(true);
            if !repo.statuses(Some(&mut options))?.is_empty() {
                anyhow::bail!("Merge target worktree must be clean");
            }
            let target_commit = head.peel_to_commit()?;
            let source_commit =
                repo.find_branch(source, BranchType::Local)?.get().peel_to_commit()?;
            let target_oid = target_commit.id();
            let source_oid = source_commit.id();
            let already_merged =
                source_oid == target_oid || repo.graph_descendant_of(target_oid, source_oid)?;
            if already_merged {
                return Ok(
                    json!({"source": source, "target": target, "commit": target_oid.to_string(), "already_merged": true}),
                );
            }
            let (commit_oid, tree) = if repo.graph_descendant_of(source_oid, target_oid)? {
                (source_oid, source_commit.tree()?)
            } else {
                // In-memory merge leaves HEAD, index and user files intact on conflicts.
                let mut index = repo.merge_commits(&target_commit, &source_commit, None)?;
                if index.has_conflicts() {
                    let paths = index
                        .conflicts()?
                        .filter_map(Result::ok)
                        .filter_map(|entry| entry.our.or(entry.their).or(entry.ancestor))
                        .map(|entry| String::from_utf8_lossy(&entry.path).into_owned())
                        .collect::<Vec<_>>();
                    return Ok(json!({"source": source, "target": target, "conflicts": paths}));
                }
                let tree = repo.find_tree(index.write_tree_to(&repo)?)?;
                let signature = Self::repository_signature(&repo)?;
                let oid = repo.commit(
                    None,
                    &signature,
                    &signature,
                    &format!("Merge {source} into {target}"),
                    &tree,
                    &[&target_commit, &source_commit],
                )?;
                (oid, tree)
            };
            let signature = Self::repository_signature(&repo)?;
            repo.checkout_tree(tree.as_object(), Some(CheckoutBuilder::new().safe()))?;
            transaction.set_target(
                &target_ref,
                commit_oid,
                Some(&signature),
                "explicit git merge",
            )?;
            transaction.commit()?;
            Ok(
                json!({"source": source, "target": target, "commit": commit_oid.to_string(), "already_merged": false}),
            )
        })();
        Ok(match result {
            Ok(value) if value.get("conflicts").is_some() => ToolResult {
                success: false,
                output: serde_json::to_string(&value)?,
                error: Some("Merge conflict; repository unchanged".into()),
            },
            Ok(value) => json_result(value),
            Err(error) => failure("Merge", error),
        })
    }
}
