//! VibeWindow welcome surface.
//! VibeWindow 欢迎界面。
//!
//! The copied authentication, trust and account onboarding steps are intentionally
//! excluded: the v3 UI only owns the product welcome presentation.
//! 此处刻意排除复制来的认证、目录信任与账户引导步骤；v3 UI 只负责产品欢迎展示。

mod welcome;

pub use welcome::WelcomeWidget;
