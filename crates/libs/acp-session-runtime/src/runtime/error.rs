//! 会话运行时错误归一化。
//!
//! 该模块把队列、ACP、超时和中断等底层错误统一包装为 `SessionRuntimeError`，
//! 并生成适合输出给调用方的结构化错误参数。

use std::error::Error as StdError;

use crate::error::AcpError;
use crate::errors::{
    QueueConnectionError, SessionModeReplayError, SessionModelReplayError,
    SessionResumeRequiredError,
};
use acp_async_control::{InterruptedError, TimeoutError};
use crate::types::{OutputErrorOrigin, OutputErrorParams};

#[cfg(test)]
#[path = "error_tests.rs"]
mod error_tests;

#[derive(Debug)]
/// 会话运行时的统一错误类型。
///
/// 它保留原始错误源、用户可见输出参数以及“输出是否已发出”等状态，便于
/// 上层在失败路径中既能返回结构化错误，又能避免重复打印同一段错误输出。
pub struct SessionRuntimeError {
    message: String,
    source: Option<Box<dyn StdError + Send + Sync + 'static>>,
    output: Option<OutputErrorParams>,
    output_already_emitted: bool,
    interrupted: bool,
}

impl SessionRuntimeError {
    /// 从底层错误构造运行时错误。
    ///
    /// 参数会被保存为错误源，并立即归一化为输出错误参数。返回值会标记错误链
    /// 中是否包含中断错误，供调用方区分主动中断和普通失败。
    pub(super) fn from_source<E>(error: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        let interrupted = is_interrupted_error(&error);
        let output = Some(normalize_runtime_error(&error, None, Some(OutputErrorOrigin::Runtime)));
        Self {
            message: error.to_string(),
            source: Some(Box::new(error)),
            output,
            output_already_emitted: false,
            interrupted,
        }
    }

    /// 标记该错误对应的输出是否已经写给调用方。
    pub(super) fn with_output_already_emitted(mut self, output_already_emitted: bool) -> Self {
        self.output_already_emitted = output_already_emitted;
        self
    }

    /// 返回可序列化的输出错误参数。
    ///
    /// 如果构造时未能生成专门参数，会回退到通用 runtime 错误。
    pub fn output_params(&self) -> OutputErrorParams {
        self.output.clone().unwrap_or_else(|| OutputErrorParams {
            code: crate::OutputErrorCode::Runtime,
            detail_code: None,
            origin: Some(OutputErrorOrigin::Runtime),
            message: self.message.clone(),
            retryable: None,
            acp: None,
            timestamp: None,
        })
    }

    /// 判断错误输出是否已经被发送。
    pub fn output_already_emitted(&self) -> bool {
        self.output_already_emitted
    }

    /// 判断错误链中是否包含主动中断错误。
    pub fn is_interrupted(&self) -> bool {
        self.interrupted
    }
}

impl std::fmt::Display for SessionRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl StdError for SessionRuntimeError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source.as_deref().map(|source| source as &(dyn StdError + 'static))
    }
}

fn is_interrupted_error(error: &(dyn StdError + 'static)) -> bool {
    let mut current = Some(error);
    while let Some(candidate) = current {
        if candidate.downcast_ref::<InterruptedError>().is_some() {
            return true;
        }
        current = candidate.source();
    }
    false
}

fn normalize_runtime_error(
    error: &(dyn StdError + 'static),
    detail_code: Option<&str>,
    origin: Option<OutputErrorOrigin>,
) -> OutputErrorParams {
    let mut current = Some(error);
    while let Some(candidate) = current {
        // 逐层检查 source 链可以保留外层上下文，同时让最具体的业务错误决定
        // 输出 code、retryable 和 ACP 细节。
        if let Some(error) = candidate.downcast_ref::<QueueConnectionError>() {
            return OutputErrorParams {
                code: error.output_code().unwrap_or(crate::OutputErrorCode::Runtime),
                detail_code: error
                    .detail_code()
                    .map(ToOwned::to_owned)
                    .or_else(|| detail_code.map(ToOwned::to_owned)),
                origin: error.origin().or(origin),
                message: error.message().to_string(),
                retryable: error.retryable(),
                acp: error.acp().cloned(),
                timestamp: None,
            };
        }
        if let Some(error) = candidate.downcast_ref::<SessionResumeRequiredError>() {
            return operational_output(error, detail_code, origin);
        }
        if let Some(error) = candidate.downcast_ref::<SessionModeReplayError>() {
            return operational_output(error, detail_code, origin);
        }
        if let Some(error) = candidate.downcast_ref::<SessionModelReplayError>() {
            return operational_output(error, detail_code, origin);
        }
        if let Some(error) = candidate.downcast_ref::<TimeoutError>() {
            return OutputErrorParams {
                code: crate::OutputErrorCode::Timeout,
                detail_code: detail_code.map(ToOwned::to_owned),
                origin,
                message: error.to_string(),
                retryable: Some(true),
                acp: None,
                timestamp: None,
            };
        }
        if let Some(error) = candidate.downcast_ref::<AcpError>() {
            return normalize_acp_error(error, detail_code, origin);
        }
        current = candidate.source();
    }

    OutputErrorParams {
        code: crate::OutputErrorCode::Runtime,
        detail_code: detail_code.map(ToOwned::to_owned),
        origin,
        message: error.to_string(),
        retryable: None,
        acp: None,
        timestamp: None,
    }
}

fn operational_output<T>(
    error: &T,
    detail_code: Option<&str>,
    origin: Option<OutputErrorOrigin>,
) -> OutputErrorParams
where
    T: std::ops::Deref<Target = crate::AcpxOperationalError>,
{
    error
        .to_output_error_params()
        .map(|mut params| {
            if params.detail_code.is_none() {
                params.detail_code = detail_code.map(ToOwned::to_owned);
            }
            if params.origin.is_none() {
                params.origin = origin;
            }
            params
        })
        .unwrap_or(OutputErrorParams {
            code: crate::OutputErrorCode::Runtime,
            detail_code: detail_code.map(ToOwned::to_owned),
            origin,
            message: error.to_string(),
            retryable: None,
            acp: None,
            timestamp: None,
        })
}

fn normalize_acp_error(
    error: &AcpError,
    detail_code: Option<&str>,
    origin: Option<OutputErrorOrigin>,
) -> OutputErrorParams {
    use crate::OutputErrorCode;

    let code = match error {
        AcpError::LoadSession(_) | AcpError::ResumeSession(_) => OutputErrorCode::NoSession,
        _ => OutputErrorCode::Runtime,
    };

    OutputErrorParams {
        code,
        detail_code: detail_code.map(ToOwned::to_owned),
        origin: origin.or(Some(OutputErrorOrigin::Acp)),
        message: error.to_string(),
        retryable: None,
        acp: None,
        timestamp: None,
    }
}
