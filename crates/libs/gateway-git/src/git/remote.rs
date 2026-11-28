//! 远端传输与安全快进。SSH 传输复用系统 Git 和 OpenSSH 配置。
use git2::{Cred, CredentialType, FetchOptions, PushOptions, RemoteCallbacks, Repository};
use std::process::Stdio;
use std::time::{Duration, Instant};
use vw_api_git::GitRemoteAction;

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
            // HTTPS 传输复用用户配置的凭据助手。
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

fn is_ssh_url(url: &[u8]) -> bool {
    if [b"ssh://".as_slice(), b"git+ssh://", b"ssh+git://"]
        .iter()
        .any(|scheme| url.starts_with(scheme))
    {
        return true;
    }
    // Git 也接受 [user@]host:path。首个斜杠在冒号前时是本地路径。
    let Some(index) = url.iter().position(|&byte| byte == b':' || byte == b'/' || byte == b'\\')
    else {
        return false;
    };
    index > 0
        && url[index] == b':'
        && !url[index + 1..].starts_with(b"//")
        && !url[index + 1..].starts_with(b":")
        && !(cfg!(windows) && index == 1 && url[0].is_ascii_alphabetic())
}

fn run_ssh_git(path: &str, args: &[&str], deadline: Instant) -> Result<(), git2::Error> {
    let mut command = shell_command::git_tokio_command();
    command
        .current_dir(path)
        .args(args)
        .env("GIT_ALLOW_PROTOCOL", "ssh")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_VARIANT", "ssh")
        .env("GIT_SSH_COMMAND", "ssh -oBatchMode=yes -oConnectTimeout=15")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    // 本函数由网关的 spawn_blocking 调用，等待子进程退出后才返回。
    tokio::runtime::Handle::current().block_on(async {
        let mut child = command
            .spawn()
            .map_err(|error| git2::Error::from_str(&format!("无法启动系统 Git：{error}")))?;
        let status = match tokio::time::timeout_at(deadline.into(), child.wait()).await {
            Ok(result) => result.map_err(|error| git2::Error::from_str(&error.to_string()))?,
            Err(_) => {
                child.kill().await.map_err(|error| git2::Error::from_str(&error.to_string()))?;
                return Err(git2::Error::from_str("Git SSH 传输超时"));
            }
        };
        if !status.success() {
            return Err(git2::Error::from_str(
                "Git SSH 操作失败，请检查网关主机的 SSH 配置、密钥及远端分支权限",
            ));
        }
        Ok(())
    })
}

fn has_multiple_push_urls(
    repo: &Repository,
    name: &str,
    has_push_url: bool,
) -> Result<bool, git2::Error> {
    // 系统 Git 会向命名远端的所有推送地址写入。保留 libgit2 的单目标行为。
    let key = if has_push_url { "pushurl" } else { "url" };
    let config = repo.config()?;
    let mut values = config.multivar(&format!("remote.{name}.{key}"), None)?;
    Ok(values.next().transpose()?.is_some() && values.next().transpose()?.is_some())
}

fn fetch(
    repo: &Repository,
    remote: &mut git2::Remote<'_>,
    path: &str,
    name: &str,
    refspecs: &[&str],
    deadline: Instant,
    reflog_message: &str,
) -> Result<(), git2::Error> {
    if is_ssh_url(remote.url_bytes()) {
        let mut args =
            vec!["fetch", "--no-recurse-submodules", "--no-auto-maintenance", "--", name];
        args.extend_from_slice(refspecs);
        return run_ssh_git(path, &args, deadline);
    }
    let mut options = FetchOptions::new();
    options.remote_callbacks(callbacks(deadline, repo.config()?));
    remote.fetch(refspecs, Some(&mut options), Some(reflog_message))
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
            fetch(&repo, &mut remote, path, name, &[], deadline, "fetch from Git panel")?;
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
            let refspec = format!("refs/heads/{branch}:refs/heads/{target}");
            let push_url = remote.pushurl_bytes();
            if is_ssh_url(push_url.unwrap_or_else(|| remote.url_bytes()))
                && !has_multiple_push_urls(&repo, name, push_url.is_some())?
            {
                run_ssh_git(
                    path,
                    &[
                        "-c",
                        "push.autoSetupRemote=false",
                        "-c",
                        &format!("remote.{name}.mirror=false"),
                        "push",
                        "--no-verify",
                        "--no-signed",
                        "--no-follow-tags",
                        "--recurse-submodules=no",
                        "--",
                        name,
                        &refspec,
                    ],
                    deadline,
                )?;
            } else {
                let mut options = PushOptions::new();
                options.remote_callbacks(callbacks(deadline, repo.config()?));
                remote.push(&[refspec], Some(&mut options))?;
            }
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
            fetch(
                &repo,
                &mut remote,
                path,
                name,
                &[&format!("refs/heads/{target}:{tracking}")],
                deadline,
                "pull from Git panel",
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
