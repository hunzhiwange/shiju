//! 安全沙箱选择、紧急停止与 OTP 控制，复用独立基础库的唯一类型所有者。

pub mod detect;
pub mod estop;
pub mod otp;

pub use process_sandbox::{docker, traits};
#[cfg(feature = "sandbox-bubblewrap")]
pub use process_sandbox::bubblewrap;
#[cfg(target_os = "linux")]
pub use process_sandbox::firejail;
#[cfg(feature = "sandbox-landlock")]
pub use process_sandbox::landlock;
pub use ::domain_matcher::domain_matcher;
pub use content_guard::leak_detector;
pub use gateway_pairing::pairing;
pub use secret_store::secrets;
#[cfg(not(target_arch = "wasm32"))]
pub use secret_store::gateway_skey_store;

pub use detect::create_sandbox;
pub use domain_matcher::DomainMatcher;
pub use estop::{EstopLevel, EstopManager, EstopState, ResumeSelector};
pub use otp::OtpValidator;
pub use secrets::SecretStore;

pub mod policy;
pub mod shell;
pub use policy::{AutonomyLevel, SecurityPolicy, ShellRedirectPolicy};

pub use content_guard::prompt_guard;
pub mod semantic_guard;
pub use semantic_guard::{GuardCorpusUpdateReport, SemanticGuard, SemanticGuardStartupStatus};

#[cfg(test)]
#[path = "prompt_guard_reexport_tests.rs"]
mod prompt_guard_reexport_tests;
