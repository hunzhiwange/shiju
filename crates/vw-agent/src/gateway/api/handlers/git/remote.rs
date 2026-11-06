//! libgit2 远端传输与安全快进；不调用 Git CLI，不强推、不自动合并或 stash。
use git2::{Cred, CredentialType, FetchOptions, PushOptions, RemoteCallbacks, Repository};
use std::time::{Duration, Instant};
use vw_api_types::git::GitRemoteAction;

const NETWORK_TIMEOUT: Duration = Duration::from_secs(120);

fn callbacks(deadline: Instant, config: git2::Config) -> RemoteCallbacks<'static> {
    let mut callbacks = RemoteCallbacks::new();
    let mut attempts = 0;
    callbacks.credentials(move |url, username, allowed| {
        attempts += 1;
        if Instant::now() > deadline || attempts > 3 {
            return Err(git2::Error::from_str("Git authentication failed or timed out"));
        }
        if allowed.contains(CredentialType::SSH_KEY) {
            return Cred::ssh_key_from_agent(username.unwrap_or("git"));
        }
        if allowed.contains(CredentialType::USERNAME) {
            return Cred::username(username.unwrap_or("git"));
        }
        if allowed.contains(CredentialType::USER_PASS_PLAINTEXT) {
            // 仅复用用户配置的认证助手；仓库数据传输始终由 libgit2 完成。
            return Cred::credential_helper(&config, url, username);
        }
        if allowed.contains(CredentialType::DEFAULT) {
            return Cred::default();
        }
        Err(git2::Error::from_str(
            "远端需要认证，请在网关配置凭据助手或 SSH agent；不支持交互式密码输入",
        ))
    });
    callbacks.transfer_progress(move |_| Instant::now() <= deadline);
    callbacks.sideband_progress(move |_| Instant::now() <= deadline);
    callbacks.push_negotiation(move |_| {
        if Instant::now() > deadline {
            return Err(git2::Error::from_str("Git transfer timed out"));
        }
        Ok(())
    });
    callbacks.push_update_reference(|_, rejection| match rejection {
        Some(reason) => Err(git2::Error::from_str(reason)),
        None => Ok(()),
    });
    callbacks
}

pub(super) fn sync(path: &str, action: GitRemoteAction) -> Result<(), String> {
    sync_inner(path, action).map_err(|error| error.to_string())
}

fn sync_inner(path: &str, action: GitRemoteAction) -> Result<(), git2::Error> {
    let repo = Repository::open(path)?;
    let snapshot =
        super::history::snapshot(path, 0).map_err(|error| git2::Error::from_str(&error))?;
    let name = snapshot
        .remote
        .as_deref()
        .ok_or_else(|| git2::Error::from_str("未找到可用远端，请先配置分支跟踪关系"))?;
    let mut remote = repo.find_remote(name)?;
    let deadline = Instant::now() + NETWORK_TIMEOUT;
    match action {
        GitRemoteAction::Fetch => {
            let mut options = FetchOptions::new();
            options.remote_callbacks(callbacks(deadline, repo.config()?));
            remote.fetch::<&str>(&[], Some(&mut options), Some("fetch from Git panel"))?;
        }
        GitRemoteAction::Push => {
            let branch = snapshot
                .branch
                .as_deref()
                .ok_or_else(|| git2::Error::from_str("分离 HEAD 或尚无提交，无法推送"))?;
            let target = snapshot
                .remote_branch
                .as_deref()
                .ok_or_else(|| git2::Error::from_str("未配置远端分支"))?;
            let mut options = PushOptions::new();
            options.remote_callbacks(callbacks(deadline, repo.config()?));
            remote
                .push(&[format!("refs/heads/{branch}:refs/heads/{target}")], Some(&mut options))?;
            if !snapshot.upstream {
                // 仅在远端接受推送后建立上游关系。
                let mut config = repo.config()?;
                config.set_str(&format!("branch.{branch}.remote"), name)?;
                config
                    .set_str(&format!("branch.{branch}.merge"), &format!("refs/heads/{target}"))?;
            }
        }
        GitRemoteAction::Pull => {
            if !snapshot.upstream {
                return Err(git2::Error::from_str("请先推送并建立上游分支，再拉取"));
            }
            ensure_clean(&repo)?;
            let head = repo.head()?;
            let refname = head.name()?.to_owned();
            let old = head.peel_to_commit()?.id();
            let target = snapshot
                .remote_branch
                .as_deref()
                .ok_or_else(|| git2::Error::from_str("未配置远端分支"))?;
            let tracking = format!("refs/remotes/{name}/{target}");
            let mut options = FetchOptions::new();
            options.remote_callbacks(callbacks(deadline, repo.config()?));
            remote.fetch(
                &[format!("refs/heads/{target}:{tracking}")],
                Some(&mut options),
                Some("pull from Git panel"),
            )?;
            let new = repo.find_reference(&tracking)?.peel_to_commit()?.id();
            if new == old || repo.graph_descendant_of(old, new)? {
                return Ok(());
            }
            if !repo.graph_descendant_of(new, old)? {
                return Err(git2::Error::from_str(
                    "本地与远端分支已分叉，无法快进拉取，请先处理分叉",
                ));
            }
            let mut transaction = repo.transaction()?;
            transaction.lock_ref(&refname)?;
            ensure_clean(&repo)?;
            let current = repo.head()?;
            if current.name()? != refname || current.target() != Some(old) {
                return Err(git2::Error::from_str("拉取期间当前分支发生变化，请刷新后重试"));
            }
            transaction.set_target(&refname, new, None, "pull: fast-forward")?;
            let tree = repo.find_commit(new)?.tree()?;
            let mut checkout = git2::build::CheckoutBuilder::new();
            checkout.safe();
            repo.checkout_tree(tree.as_object(), Some(&mut checkout))?;
            transaction.commit()?;
        }
    }
    Ok(())
}

fn ensure_clean(repo: &Repository) -> Result<(), git2::Error> {
    let mut options = git2::StatusOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(true);
    if repo.state() != git2::RepositoryState::Clean
        || !repo.statuses(Some(&mut options))?.is_empty()
    {
        return Err(git2::Error::from_str(
            "工作区存在未提交变更或进行中的 Git 操作，请处理后再拉取",
        ));
    }
    Ok(())
}
