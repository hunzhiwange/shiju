//! # 运行时适配器测试模块
//!
//! 本模块提供针对运行时适配器工厂函数的单元测试，验证各种运行时类型的创建行为、
//! 功能特性以及错误处理机制。
//!
//! ## 测试覆盖范围
//!
//! - **有效运行时类型测试**：验证 native、docker 两种运行时的正确创建
//! - **错误处理测试**：验证未实现、未知类型、空字符串等错误场景的处理
//!
//! ## 测试策略
//!
//! 采用黑盒测试方法，通过 `create_runtime` 工厂函数创建不同配置的运行时实例，
//! 验证返回的运行时对象是否符合预期的类型名称和能力特性。

use super::*;

/// 运行时适配器测试套件
///
/// 包含所有运行时创建相关的测试用例，覆盖正向场景和错误处理场景。
#[allow(dead_code)]
mod tests {
    use super::*;

    /// 测试创建 native 运行时适配器
    ///
    /// 验证使用 "native" 类型配置时，能够成功创建运行时实例，并具备以下特性：
    /// - 运行时名称为 "native"
    /// - 拥有 shell 访问权限（可执行系统命令）
    ///
    /// # 测试步骤
    /// 1. 创建 kind 为 "native" 的运行时配置
    /// 2. 调用工厂函数创建运行时实例
    /// 3. 断言运行时名称为 "native"
    /// 4. 断言具有 shell 访问权限
    #[test]
    fn factory_native() {
        let cfg = RuntimeConfig { kind: "native".into(), ..RuntimeConfig::default() };
        let rt = create_runtime(&cfg).unwrap();
        assert_eq!(rt.name(), "native");
        assert!(rt.has_shell_access());
    }

    /// 测试创建 docker 运行时适配器
    ///
    /// 验证使用 "docker" 类型配置时，能够成功创建运行时实例，并具备以下特性：
    /// - 运行时名称为 "docker"
    /// - 拥有 shell 访问权限（通过容器接口执行命令）
    ///
    /// # 测试步骤
    /// 1. 创建 kind 为 "docker" 的运行时配置
    /// 2. 调用工厂函数创建运行时实例
    /// 3. 断言运行时名称为 "docker"
    /// 4. 断言具有 shell 访问权限
    #[test]
    fn factory_docker() {
        let cfg = RuntimeConfig { kind: "docker".into(), ..RuntimeConfig::default() };
        let rt = create_runtime(&cfg).unwrap();
        assert_eq!(rt.name(), "docker");
        assert!(rt.has_shell_access());
    }

    /// 测试创建 cloudflare 运行时适配器应返回错误
    ///
    /// 验证使用 "cloudflare" 类型配置时，工厂函数应返回错误而非成功创建实例。
    /// Cloudflare 运行时目前尚未实现，应返回包含 "not implemented" 的错误信息。
    ///
    /// # 测试步骤
    /// 1. 创建 kind 为 "cloudflare" 的运行时配置
    /// 2. 调用工厂函数创建运行时实例
    /// 3. 断言返回错误（而非成功）
    /// 4. 断言错误信息包含 "not implemented"
    ///
    /// # 期望行为
    /// 返回 `Err`，错误信息包含 "not implemented" 字样
    #[test]
    fn factory_cloudflare_errors() {
        let cfg = RuntimeConfig { kind: "cloudflare".into(), ..RuntimeConfig::default() };
        match create_runtime(&cfg) {
            Err(err) => assert!(err.to_string().contains("not implemented")),
            Ok(_) => panic!("cloudflare runtime should error"),
        }
    }

    /// 测试创建未知类型运行时适配器应返回错误
    ///
    /// 验证使用不存在的运行时类型名称时，工厂函数应返回明确的错误信息。
    /// 错误信息应包含 "Unknown runtime kind" 以便于诊断问题。
    ///
    /// # 测试步骤
    /// 1. 创建 kind 为 "unsupported" 的运行时配置（不存在的类型）
    /// 2. 调用工厂函数创建运行时实例
    /// 3. 断言返回错误（而非成功）
    /// 4. 断言错误信息包含 "Unknown runtime kind"
    ///
    /// # 期望行为
    /// 返回 `Err`，错误信息包含 "Unknown runtime kind" 字样
    #[test]
    fn factory_unknown_errors() {
        let cfg = RuntimeConfig { kind: "unsupported".into(), ..RuntimeConfig::default() };
        match create_runtime(&cfg) {
            Err(err) => assert!(err.to_string().contains("Unknown runtime kind")),
            Ok(_) => panic!("unknown runtime should error"),
        }
    }

    /// 测试创建空字符串类型运行时适配器应返回错误
    ///
    /// 验证运行时类型配置为空字符串时，工厂函数应拒绝创建并返回明确的错误信息。
    /// 错误信息应包含 "cannot be empty" 以提示配置问题。
    ///
    /// # 测试步骤
    /// 1. 创建 kind 为空字符串的运行时配置
    /// 2. 调用工厂函数创建运行时实例
    /// 3. 断言返回错误（而非成功）
    /// 4. 断言错误信息包含 "cannot be empty"
    ///
    /// # 期望行为
    /// 返回 `Err`，错误信息包含 "cannot be empty" 字样
    ///
    /// # 安全性考虑
    /// 空字符串类型配置通常表示配置缺失或解析错误，应及早失败而非使用默认值，
    /// 以避免隐蔽的安全风险。
    #[test]
    fn factory_empty_errors() {
        let cfg = RuntimeConfig { kind: String::new(), ..RuntimeConfig::default() };
        match create_runtime(&cfg) {
            Err(err) => assert!(err.to_string().contains("cannot be empty")),
            Ok(_) => panic!("empty runtime should error"),
        }
    }

    #[test]
    fn factory_whitespace_kind_errors_as_empty() {
        let cfg = RuntimeConfig { kind: " \t\n ".into(), ..RuntimeConfig::default() };
        let err = create_runtime(&cfg).err().expect("whitespace runtime kind should error");
        assert!(err.to_string().contains("cannot be empty"));
    }

    #[test]
    fn factory_returns_downcastable_runtime_types() {
        let docker = RuntimeConfig { kind: "docker".into(), ..RuntimeConfig::default() };
        let docker_rt = create_runtime(&docker).unwrap();
        assert!(docker_rt.as_any().downcast_ref::<DockerRuntime>().is_some());
    }
}
