//! 带请求代次的领域资源加载状态。

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// 通用异步资源加载状态，加载分支携带请求代次。
pub enum LoadState<T> {
    #[default]
    Idle,
    NeedsLogin,
    Loading {
        generation: u64,
    },
    Loaded(T),
    Error(super::SkillHubError),
}
