//! Local skill source discovery, ordered by workspace, ancestor and global precedence.
use config_skills::{PROJECT_SKILL_SOURCE_PATHS, PROJECT_SKILLS_DIR};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalSkillSourceKind {
    Workspace,
    Ancestor,
    Global,
}

#[derive(Debug, Clone)]
pub struct LocalSkillSourceDir {
    pub kind: LocalSkillSourceKind,
    pub path: PathBuf,
}

pub fn discover_local_skill_source_dirs(workspace_dir: Option<&Path>) -> Vec<LocalSkillSourceDir> {
    let mut dirs = Vec::new();
    let mut seen = HashSet::new();
    // Windows 通常只设置 USERPROFILE；保留 OsString，避免损坏中文用户名路径。
    let home_dir = std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .or_else(|| std::env::var_os("USERPROFILE").filter(|value| !value.is_empty()))
        .map(PathBuf::from);

    if let Some(workspace_dir) = workspace_dir {
        let workspace_dir =
            workspace_dir.canonicalize().unwrap_or_else(|_| workspace_dir.to_path_buf());

        for relative_path in PROJECT_SKILL_SOURCE_PATHS {
            push_skill_source_dir(
                &mut dirs,
                &mut seen,
                LocalSkillSourceKind::Workspace,
                workspace_dir.join(relative_path),
            );
        }

        let stop_dir = discover_ancestor_scan_stop(&workspace_dir, home_dir.as_deref());
        if stop_dir.as_ref() != Some(&workspace_dir) {
            let mut current = workspace_dir.parent().map(Path::to_path_buf);
            while let Some(dir) = current {
                if home_dir.as_ref().is_some_and(|home| dir == home.as_path()) {
                    break;
                }

                for relative_path in PROJECT_SKILL_SOURCE_PATHS {
                    push_skill_source_dir(
                        &mut dirs,
                        &mut seen,
                        LocalSkillSourceKind::Ancestor,
                        dir.join(relative_path),
                    );
                }

                if stop_dir.as_ref().is_some_and(|stop| dir == stop.as_path()) {
                    break;
                }

                current = dir.parent().map(Path::to_path_buf);
            }
        }
    }

    if let Some(home_dir) = home_dir {
        push_skill_source_dir(
            &mut dirs,
            &mut seen,
            LocalSkillSourceKind::Global,
            config_runtime_paths::home_config_dir(&home_dir).join("skills"),
        );
        push_skill_source_dir(
            &mut dirs,
            &mut seen,
            LocalSkillSourceKind::Global,
            home_dir.join(".skills"),
        );
    }

    dirs
}

pub fn workspace_skills_dir(workspace_dir: &Path) -> PathBuf {
    workspace_dir.join(PROJECT_SKILLS_DIR)
}

fn push_skill_source_dir(
    dirs: &mut Vec<LocalSkillSourceDir>,
    seen: &mut HashSet<PathBuf>,
    kind: LocalSkillSourceKind,
    path: PathBuf,
) {
    let realpath = path.canonicalize().unwrap_or_else(|_| path.clone());
    if path.is_dir() && seen.insert(realpath) {
        dirs.push(LocalSkillSourceDir { kind, path });
    }
}

fn discover_ancestor_scan_stop(workspace_dir: &Path, home_dir: Option<&Path>) -> Option<PathBuf> {
    let mut current = Some(workspace_dir);
    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return Some(dir.to_path_buf());
        }
        if home_dir.is_some_and(|home| dir == home) {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}
