//! 函数包装器工具模块
//!
//! 本模块提供了一种将解析和执行分离的函数包装器模式。
//! 通过 `FnWrap` 可以将一个解析函数和一个执行函数组合在一起，
//! 实现先验证/解析输入，再执行业务逻辑的流程。
//!
//! # 设计模式
//!
//! - **Parse 函数**: 负责输入的验证和转换，失败时返回错误
//! - **Call 函数**: 负责实际业务执行，接收已验证/解析的数据
//!
//! # 使用场景
//!
//! - 命令行参数解析与执行
//! - 请体验证与处理
//! - 配置解析与应用

use std::marker::PhantomData;

/// 函数包装器，将解析和执行两阶段组合
///
/// `FnWrap` 持有两个闭包：
/// 1. `parse`: 将原始输入转换为已验证/解析的形式
/// 2. `call`: 执行实际业务逻辑
///
/// # 类型参数
///
/// - `Input`: 原始输入类型
/// - `Parsed`: 解析后的数据类型
/// - `Output`: 执行结果类型
/// - `Error`: 解析错误类型
/// - `Parse`: 解析函数类型，`Input -> Result<Parsed, Error>`
/// - `Call`: 执行函数类型，`Parsed -> Output`
///
/// # 示例
///
/// ```ignore
/// use agent_context::r#fn::{fn_wrap, FnWrap};
///
/// // 解析函数：验证字符串是否为有效数字
/// fn parse_number(s: String) -> Result<i32, String> {
///     s.parse::<i32>().map_err(|_| "无效数字".to_string())
/// }
///
/// // 执行函数：计算平方
/// fn square(n: i32) -> i32 {
///     n * n
/// }
///
/// let wrapper = fn_wrap(parse_number, square);
/// assert_eq!(wrapper.call("4".to_string()), Ok(16));
/// assert!(wrapper.call("abc".to_string()).is_err());
/// ```
pub struct FnWrap<Input, Parsed, Output, Error, Parse, Call>
where
    Parse: Fn(Input) -> Result<Parsed, Error>,
    Call: Fn(Parsed) -> Output,
{
    /// 解析函数：输入验证与转换
    parse: Parse,
    /// 执行函数：业务逻辑处理
    call: Call,
    /// 幻影数据，用于携带类型参数而不占用实际内存
    _marker: PhantomData<(Input, Parsed, Output, Error)>,
}

/// 创建函数包装器
///
/// 将解析函数和执行函数组合成一个 `FnWrap` 实例。
///
/// # 参数
///
/// - `parse`: 解析函数，接收原始输入并返回解析结果或错误
/// - `call`: 执行函数，接收解析后的数据并返回执行结果
///
/// # 返回值
///
/// 返回一个 `FnWrap` 实例，可用于执行完整的解析-执行流程
///
/// # 示例
///
/// ```ignore
/// let wrapper = fn_wrap(
///     |s: String| s.parse::<i32>().map_err(|_| "解析失败"),
///     |n: i32| n * 2,
/// );
/// assert_eq!(wrapper.call("10".to_string()), Ok(20));
/// ```
pub fn fn_wrap<Input, Parsed, Output, Error, Parse, Call>(
    parse: Parse,
    call: Call,
) -> FnWrap<Input, Parsed, Output, Error, Parse, Call>
where
    Parse: Fn(Input) -> Result<Parsed, Error>,
    Call: Fn(Parsed) -> Output,
{
    // 使用幻影数据携带类型信息，避免编译器警告未使用的类型参数
    FnWrap { parse, call, _marker: PhantomData }
}

impl<Input, Parsed, Output, Error, Parse, Call> FnWrap<Input, Parsed, Output, Error, Parse, Call>
where
    Parse: Fn(Input) -> Result<Parsed, Error>,
    Call: Fn(Parsed) -> Output,
{
    /// 执行完整的解析-执行流程
    ///
    /// 先调用解析函数验证和转换输入，成功后再调用执行函数。
    ///
    /// # 参数
    ///
    /// - `input`: 原始输入数据
    ///
    /// # 返回值
    ///
    /// - `Ok(Output)`: 解析和执行都成功时的结果
    /// - `Err(Error)`: 解析失败时的错误
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let wrapper = fn_wrap(
    ///     |s: String| s.parse::<i32>().map_err(|_| "无效数字"),
    ///     |n: i32| format!("结果: {}", n),
    /// );
    ///
    /// assert_eq!(wrapper.call("42".to_string()), Ok("结果: 42".to_string()));
    /// assert!(wrapper.call("xyz".to_string()).is_err());
    /// ```
    pub fn call(&self, input: Input) -> Result<Output, Error> {
        // 第一步：解析/验证输入，失败则提前返回错误
        let parsed = (self.parse)(input)?;
        // 第二步：执行业务逻辑并包装为成功结果
        Ok((self.call)(parsed))
    }

    /// 强制执行（跳过解析阶段）
    ///
    /// 直接调用执行函数，不经过解析步骤。
    /// 适用于调用者已确保数据有效，或需要重复执行的场景。
    ///
    /// # 参数
    ///
    /// - `input`: 已解析的数据
    ///
    /// # 返回值
    ///
    /// 返回执行函数的结果
    ///
    /// # 安全性
    ///
    /// 调用者需自行确保输入数据有效，此方法不执行任何验证。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let wrapper = fn_wrap(
    ///     |s: String| s.parse::<i32>().map_err(|_| "无效数字"),
    ///     |n: i32| n * 2,
    /// );
    ///
    /// // 直接传入已解析的数据
    /// assert_eq!(wrapper.force(10), 20);
    /// ```
    pub fn force(&self, input: Parsed) -> Output {
        // 直接执行，不经过解析步骤
        (self.call)(input)
    }
}
