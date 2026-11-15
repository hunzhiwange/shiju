//! 通道配置辅助函数模块
//!
//! 本模块提供通道配置处理过程中使用的辅助函数，主要用于：
//! - 环境变量名称的合法性验证
//! - 敏感数据的加密与解密操作
//!
//! # 主要功能
//!
//! 1. **环境变量验证**：验证环境变量名称是否符合命名规范
//! 2. **密钥加密/解密**：提供可选和必需两种形式的密钥加密解密函数
//!
//! # 安全性
//!
//! 所有涉及密钥的函数都使用 `SecretStore` 进行安全处理，确保敏感信息
//! 在配置文件中以加密形式存储。

use secret_store::secrets::SecretStore;
use anyhow::{Context, Result};

/// 验证环境变量名称是否合法
///
/// 检查给定的字符串是否符合环境变量命名规范：
/// - 必须以字母（a-z, A-Z）或下划线（_）开头
/// - 后续字符可以是字母、数字（0-9）或下划线
///
/// # 参数
///
/// * `name` - 待验证的环境变量名称字符串
///
/// # 返回值
///
/// 返回 `true` 表示名称合法，`false` 表示名称不合法
///
/// # 示例
///
/// ```ignore
/// use crate::schema::channels::helpers::is_valid_env_var_name;
///
/// assert!(is_valid_env_var_name("API_KEY"));      // 合法
/// assert!(is_valid_env_var_name("_SECRET"));      // 合法
/// assert!(is_valid_env_var_name("DB2_NAME"));     // 合法
/// assert!(!is_valid_env_var_name("2FAST"));       // 非法：以数字开头
/// assert!(!is_valid_env_var_name("MY-VAR"));      // 非法：包含连字符
/// assert!(!is_valid_env_var_name(""));            // 非法：空字符串
/// ```
pub(crate) fn is_valid_env_var_name(name: &str) -> bool {
    let mut chars = name.chars();
    // 检查第一个字符：必须是字母或下划线
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }
    // 检查剩余字符：必须是字母、数字或下划线
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

/// 解密可选的密钥字段
///
/// 对可能包含加密内容的可选字符串字段进行解密。如果字段值为 `None`，
/// 则不做任何处理。如果字段值已加密（通过 `SecretStore::is_encrypted` 判断），
/// 则进行解密并更新原值。
///
/// # 参数
///
/// * `store` - 密钥存储实例，用于执行解密操作
/// * `value` - 可变引用指向待解密的可选字符串值
/// * `field_name` - 字段名称，用于错误信息中标识具体字段
///
/// # 返回值
///
/// * `Ok(())` - 解密成功或无需解密
/// * `Err` - 解密失败，错误信息包含字段名称
///
/// # 示例
///
/// ```ignore
/// use secret_store::secrets::SecretStore;
/// use crate::schema::channels::helpers::decrypt_optional_secret;
///
/// let store = SecretStore::new("encryption_key")?;
/// let mut value = Some("encrypted:abc123".to_string());
/// decrypt_optional_secret(&store, &mut value, "api_key")?;
/// // value 现在包含解密后的明文
/// ```
pub fn decrypt_optional_secret(
    store: &SecretStore,
    value: &mut Option<String>,
    field_name: &str,
) -> Result<()> {
    if let Some(raw) = value.clone() {
        // 仅当值已加密时才进行解密
        if SecretStore::is_encrypted(&raw) {
            *value = Some(
                store.decrypt(&raw).with_context(|| format!("Failed to decrypt {field_name}"))?,
            );
        }
    }
    Ok(())
}

/// 解密必需的密钥字段
///
/// 对包含加密内容的字符串字段进行解密。如果字段值已加密
/// （通过 `SecretStore::is_encrypted` 判断），则进行解密并更新原值。
///
/// # 参数
///
/// * `store` - 密钥存储实例，用于执行解密操作
/// * `value` - 可变引用指向待解密的字符串值
/// * `field_name` - 字段名称，用于错误信息中标识具体字段
///
/// # 返回值
///
/// * `Ok(())` - 解密成功或无需解密
/// * `Err` - 解密失败，错误信息包含字段名称
///
/// # 示例
///
/// ```ignore
/// use secret_store::secrets::SecretStore;
/// use crate::schema::channels::helpers::decrypt_secret;
///
/// let store = SecretStore::new("encryption_key")?;
/// let mut value = "encrypted:abc123".to_string();
/// decrypt_secret(&store, &mut value, "password")?;
/// // value 现在包含解密后的明文
/// ```
pub fn decrypt_secret(
    store: &SecretStore,
    value: &mut String,
    field_name: &str,
) -> Result<()> {
    // 仅当值已加密时才进行解密
    if SecretStore::is_encrypted(value) {
        *value = store.decrypt(value).with_context(|| format!("Failed to decrypt {field_name}"))?;
    }
    Ok(())
}

/// 加密可选的密钥字段
///
/// 对可能包含明文敏感数据的可选字符串字段进行加密。如果字段值为 `None`，
/// 则不做任何处理。如果字段值未加密（通过 `SecretStore::is_encrypted` 判断），
/// 则进行加密并更新原值。
///
/// # 参数
///
/// * `store` - 密钥存储实例，用于执行加密操作
/// * `value` - 可变引用指向待加密的可选字符串值
/// * `field_name` - 字段名称，用于错误信息中标识具体字段
///
/// # 返回值
///
/// * `Ok(())` - 加密成功或无需加密
/// * `Err` - 加密失败，错误信息包含字段名称
///
/// # 示例
///
/// ```ignore
/// use secret_store::secrets::SecretStore;
/// use crate::schema::channels::helpers::encrypt_optional_secret;
///
/// let store = SecretStore::new("encryption_key")?;
/// let mut value = Some("my_secret_password".to_string());
/// encrypt_optional_secret(&store, &mut value, "password")?;
/// // value 现在包含加密后的密文
/// ```
pub fn encrypt_optional_secret(
    store: &SecretStore,
    value: &mut Option<String>,
    field_name: &str,
) -> Result<()> {
    if let Some(raw) = value.clone() {
        // 仅当值未加密时才进行加密，避免重复加密
        if !SecretStore::is_encrypted(&raw) {
            *value = Some(
                store.encrypt(&raw).with_context(|| format!("Failed to encrypt {field_name}"))?,
            );
        }
    }
    Ok(())
}

/// 加密必需的密钥字段
///
/// 对包含明文敏感数据的字符串字段进行加密。如果字段值未加密
/// （通过 `SecretStore::is_encrypted` 判断），则进行加密并更新原值。
///
/// # 参数
///
/// * `store` - 密钥存储实例，用于执行加密操作
/// * `value` - 可变引用指向待加密的字符串值
/// * `field_name` - 字段名称，用于错误信息中标识具体字段
///
/// # 返回值
///
/// * `Ok(())` - 加密成功或无需加密
/// * `Err` - 加密失败，错误信息包含字段名称
///
/// # 示例
///
/// ```ignore
/// use secret_store::secrets::SecretStore;
/// use crate::schema::channels::helpers::encrypt_secret;
///
/// let store = SecretStore::new("encryption_key")?;
/// let mut value = "my_api_key".to_string();
/// encrypt_secret(&store, &mut value, "api_key")?;
/// // value 现在包含加密后的密文
/// ```
pub fn encrypt_secret(
    store: &SecretStore,
    value: &mut String,
    field_name: &str,
) -> Result<()> {
    // 仅当值未加密时才进行加密，避免重复加密
    if !SecretStore::is_encrypted(value) {
        *value = store.encrypt(value).with_context(|| format!("Failed to encrypt {field_name}"))?;
    }
    Ok(())
}
