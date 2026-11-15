//! 身份系统模块，支持 OpenClaw（markdown）和 AIEOS（JSON）两种格式。
//!
//! AIEOS（AI 实体对象规范）是一个用于可移植 AI 身份的标准化框架。
//! 本模块负责加载 AIEOS v1.1 JSON 格式，并将其转换为 VibeWindow 的系统提示格式。
//!
//! # 主要功能
//!
//! - 从配置文件或内联 JSON 加载 AIEOS 身份定义
//! - 解析和规范化 AIEOS 各个数据段
//! - 将 AIEOS 身份转换为适用于 AI 代理的系统提示文本
//!
//! # 数据结构
//!
//! - [`AieosIdentity`]: AIEOS v1.1 的顶层身份结构
//! - 各个 Section 结构体：分别表示身份的不同维度（心理、语言、动机等）
//!
//! # 示例
//!
//! ```ignore
//! use crate::app::agent::identity::{load_aieos_identity, aieos_to_system_prompt};
//!
//! // 从配置加载身份
//! let identity = load_aieos_identity(&config, &workspace_dir)?;
//!
//! // 转换为系统提示
//! if let Some(id) = identity {
//!     let prompt = aieos_to_system_prompt(&id);
//! }
//! ```

pub use agent_identity_loader::{is_aieos_configured, load_aieos_identity};
pub use agent_identity::{
    AieosIdentity, CapabilitiesSection, HistorySection, IdentitySection, InterestsSection,
    LinguisticsSection, MotivationsSection, Names, OceanTraits, PhysicalitySection,
    PsychologySection,
};
pub use agent_identity::aieos_to_system_prompt;
