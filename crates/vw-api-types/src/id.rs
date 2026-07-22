//! 领域实体 ID 的轻量包装类型。
//!
//! 本模块用字符串新类型包装不同业务域的标识符，目标是：
//! - 在 Rust 类型系统中区分会话、项目、消息、任务等 ID
//! - 保持序列化形式仍然是字符串，兼容现有协议
//! - 降低把一种 ID 误传到另一种字段中的风险
//!
//! 普通 ID 基于同一宏生成；租户 ID 额外在构造和反序列化时拒绝空白值。

use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! string_id {
    ($name:ident) => {
        /// 基于字符串的稳定标识符新类型。
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
    };
}

/// 作用域 ID 为空时的校验错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdValidationError {
    field_name: &'static str,
}

impl IdValidationError {
    fn new(field_name: &'static str) -> Self {
        Self { field_name }
    }
}

impl fmt::Display for IdValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} must not be empty", self.field_name)
    }
}

impl std::error::Error for IdValidationError {}

macro_rules! validated_string_id {
    ($name:ident, $field_name:literal) => {
        /// 经非空校验的稳定字符串标识符。
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, IdValidationError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(IdValidationError::new($field_name));
                }
                Ok(Self(value))
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::new(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

validated_string_id!(PlatformId, "platform id");
validated_string_id!(CompanyId, "company id");

/// 缺少显式配置时使用的平台 ID。
pub const DEFAULT_PLATFORM_ID: &str = "10000";
/// 缺少显式配置时使用的公司 ID。
pub const DEFAULT_COMPANY_ID: &str = "100000000";
/// 网关请求携带平台 ID 的 header 名。
pub const PLATFORM_ID_HEADER: &str = "X-Platform-ID";
/// 网关请求携带公司 ID 的 header 名。
pub const COMPANY_ID_HEADER: &str = "X-Company-ID";

/// 平台和公司组成的租户作用域。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantScope {
    platform_id: PlatformId,
    company_id: CompanyId,
}

impl TenantScope {
    pub fn new(
        platform_id: impl Into<String>,
        company_id: impl Into<String>,
    ) -> Result<Self, IdValidationError> {
        Ok(Self {
            platform_id: PlatformId::new(platform_id)?,
            company_id: CompanyId::new(company_id)?,
        })
    }

    pub fn platform_id(&self) -> &PlatformId {
        &self.platform_id
    }

    pub fn company_id(&self) -> &CompanyId {
        &self.company_id
    }
}

impl Default for TenantScope {
    fn default() -> Self {
        Self {
            platform_id: PlatformId(DEFAULT_PLATFORM_ID.to_owned()),
            company_id: CompanyId(DEFAULT_COMPANY_ID.to_owned()),
        }
    }
}

/// 租户和项目 ID 组成的项目作用域。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectScope {
    #[serde(flatten)]
    tenant_scope: TenantScope,
    #[serde(deserialize_with = "deserialize_non_empty_project_id")]
    project_id: ProjectId,
}

impl ProjectScope {
    pub fn new(
        tenant_scope: TenantScope,
        project_id: ProjectId,
    ) -> Result<Self, IdValidationError> {
        if project_id.as_ref().trim().is_empty() {
            return Err(IdValidationError::new("project id"));
        }
        Ok(Self { tenant_scope, project_id })
    }

    pub fn for_default_tenant(project_id: ProjectId) -> Result<Self, IdValidationError> {
        Self::new(TenantScope::default(), project_id)
    }

    pub fn tenant_scope(&self) -> &TenantScope {
        &self.tenant_scope
    }

    pub fn platform_id(&self) -> &PlatformId {
        self.tenant_scope.platform_id()
    }

    pub fn company_id(&self) -> &CompanyId {
        self.tenant_scope.company_id()
    }

    pub fn project_id(&self) -> &ProjectId {
        &self.project_id
    }
}

fn deserialize_non_empty_project_id<'de, D>(deserializer: D) -> Result<ProjectId, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let project_id = ProjectId::deserialize(deserializer)?;
    if project_id.as_ref().trim().is_empty() {
        return Err(serde::de::Error::custom(IdValidationError::new("project id")));
    }
    Ok(project_id)
}

string_id!(MessageId);
string_id!(ModelId);
string_id!(ProjectId);
string_id!(ProviderId);
string_id!(QuestionId);
string_id!(RequestId);
string_id!(SessionId);
string_id!(TaskId);
string_id!(TodoId);
string_id!(ToolId);
string_id!(WorktreeId);
